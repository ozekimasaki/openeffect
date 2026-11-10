use std::collections::BTreeMap;

use crate::ast::{BinOp, Expr, Stmt, UnaryOp};
use crate::legacy::precheck;
use crate::parse::parse_program;
use crate::scene::Scene;
use crate::value::{Function, Value};
use crate::{host, ExprFailure, Target};

pub const UNDEFINED_VALUE: &str =
    "Undefined value used in expression (could be an out of range array subscript?)";
pub const OBJECT_WHERE_VALUE_NEEDED: &str =
    "Object of type found where a Number, Array, or Property is needed";

struct Flow {
    value: Value,
    returned: bool,
}

pub struct Ctx<'a> {
    pub scene: &'a Scene,
    pub target: Target,
    pub scopes: Vec<BTreeMap<String, Value>>,
    pub in_eval: bool,
    pub poster_fps: Option<f64>,
    pub seed_offset: f64,
    pub timeless: bool,
    pub rng_counter: u64,
}

impl<'a> Ctx<'a> {
    pub fn now(&self) -> f64 {
        match self.poster_fps {
            Some(fps) if fps == 0.0 => 0.0,
            Some(fps) if fps > 0.0 => (self.target.time * fps).floor() / fps,
            _ => self.target.time,
        }
    }

    pub fn declare(&mut self, name: &str, value: Value) {
        self.scopes
            .last_mut()
            .expect("scope")
            .insert(name.to_string(), value);
    }

    pub fn set_var(&mut self, name: &str, value: Value) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return Ok(());
            }
        }
        self.declare(name, value);
        Ok(())
    }

    pub fn lookup_var(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }
}

pub fn execute(
    scene: &Scene,
    target: Target,
    source: &str,
    in_eval: bool,
) -> Result<Value, ExprFailure> {
    let _javascript = match scene.engine_mode {
        crate::scene::EngineMode::Javascript | crate::scene::EngineMode::Legacy => true,
    };
    if !_javascript {
        return Err(ExprFailure {
            message: "expression engine is off".into(),
        });
    }
    run(scene, target, source, in_eval).map_err(|message| ExprFailure { message })
}

fn run(scene: &Scene, target: Target, source: &str, in_eval: bool) -> Result<Value, String> {
    precheck(source)?;
    let stmts = parse_program(source)?;
    if finishes_with_open_if(&stmts) {
        return Err(UNDEFINED_VALUE.into());
    }
    if finishes_with_function(&stmts) {
        return Err(OBJECT_WHERE_VALUE_NEEDED.into());
    }
    let mut ctx = Ctx {
        scene,
        target,
        scopes: vec![BTreeMap::new()],
        in_eval,
        poster_fps: None,
        seed_offset: 0.0,
        timeless: false,
        rng_counter: 0,
    };
    hoist(&mut ctx, &stmts);
    let flow = exec_stmts(&mut ctx, &stmts)?;
    host::unwrap_property(&ctx, flow.value)
}

impl<'a> Ctx<'a> {
    pub fn eval_nested(&mut self, source: &str) -> Result<Value, String> {
        if source.contains("JSXBIN") {
            return Err("jsxbin is not evaluated".into());
        }
        precheck(source)?;
        let stmts = parse_program(source)?;
        if finishes_with_open_if(&stmts) {
            return Err(UNDEFINED_VALUE.into());
        }
        if finishes_with_function(&stmts) {
            return Err(OBJECT_WHERE_VALUE_NEEDED.into());
        }
        let previous = self.in_eval;
        self.in_eval = true;
        hoist(self, &stmts);
        let flow = exec_stmts(self, &stmts);
        self.in_eval = previous;
        host::unwrap_property(self, flow?.value)
    }
}

fn hoist(ctx: &mut Ctx<'_>, stmts: &[Stmt]) {
    for stmt in stmts {
        if let Stmt::Fn { name, params, body } = stmt {
            ctx.declare(
                name,
                Value::Function(Function {
                    params: params.clone(),
                    body: body.clone(),
                }),
            );
        }
    }
}

fn finishes_with_open_if(stmts: &[Stmt]) -> bool {
    matches!(stmts.last(), Some(Stmt::If { no: None, .. }))
}

fn finishes_with_function(stmts: &[Stmt]) -> bool {
    matches!(stmts.last(), Some(Stmt::Fn { .. }))
}

fn exec_stmts(ctx: &mut Ctx<'_>, stmts: &[Stmt]) -> Result<Flow, String> {
    let mut flow = Flow {
        value: Value::Null,
        returned: false,
    };
    for stmt in stmts {
        flow = exec_stmt(ctx, stmt)?;
        if flow.returned {
            break;
        }
    }
    Ok(flow)
}

fn eval_unwrapped(ctx: &mut Ctx<'_>, expr: &Expr) -> Result<Value, String> {
    let value = eval_expr(ctx, expr)?;
    host::unwrap_property(ctx, value)
}

fn exec_stmt(ctx: &mut Ctx<'_>, stmt: &Stmt) -> Result<Flow, String> {
    match stmt {
        Stmt::Var { name, init } => {
            let value = eval_expr(ctx, init)?;
            let value = host::unwrap_property(ctx, value)?;
            ctx.declare(name, value.clone());
            Ok(Flow {
                value,
                returned: false,
            })
        }
        Stmt::Fn { .. } => Ok(Flow {
            value: Value::Null,
            returned: false,
        }),
        Stmt::If { cond, yes, no, .. } => {
            let cond = eval_expr(ctx, cond)?;
            let cond = host::unwrap_property(ctx, cond)?;
            if cond.truthy() {
                exec_stmt(ctx, yes)
            } else if let Some(no) = no {
                exec_stmt(ctx, no)
            } else {
                Ok(Flow {
                    value: Value::Null,
                    returned: false,
                })
            }
        }
        Stmt::Return(expr) => {
            let value = match expr {
                Some(expr) => eval_unwrapped(ctx, expr)?,
                None => Value::Null,
            };
            Ok(Flow {
                value,
                returned: true,
            })
        }
        Stmt::Block(body) => exec_stmts(ctx, body),
        Stmt::Expr(expr) => Ok(Flow {
            value: eval_expr(ctx, expr)?,
            returned: false,
        }),
    }
}

fn eval_expr(ctx: &mut Ctx<'_>, expr: &Expr) -> Result<Value, String> {
    match expr {
        Expr::Number(value) => Ok(Value::Number(*value)),
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Bool(value) => Ok(Value::Bool(*value)),
        Expr::Null => Ok(Value::Null),
        Expr::Ident(name) => eval_ident(ctx, name),
        Expr::Array(items) => {
            let mut values = Vec::new();
            for item in items {
                values.push(eval_unwrapped(ctx, item)?);
            }
            Ok(Value::Array(values))
        }
        Expr::Unary { op, expr } => {
            let value = eval_unwrapped(ctx, expr)?;
            match op {
                UnaryOp::Neg => Ok(Value::Number(-host::need_num(&value)?)),
                UnaryOp::Plus => Ok(Value::Number(host::need_num(&value)?)),
                UnaryOp::Not => Ok(Value::Bool(!value.truthy())),
            }
        }
        Expr::Binary { op, left, right } => eval_binary(ctx, *op, left, right),
        Expr::Assign { name, expr } => {
            let value = eval_unwrapped(ctx, expr)?;
            ctx.set_var(name, value.clone())?;
            Ok(value)
        }
        Expr::Ternary { cond, yes, no } => {
            let cond = eval_unwrapped(ctx, cond)?;
            if cond.truthy() {
                eval_expr(ctx, yes)
            } else {
                eval_expr(ctx, no)
            }
        }
        Expr::Member { object, name } => {
            let object = eval_expr(ctx, object)?;
            host::member(ctx, object, name)
        }
        Expr::Index { object, index } => {
            let object = eval_expr(ctx, object)?;
            let index = eval_unwrapped(ctx, index)?;
            host::index(ctx, object, index)
        }
        Expr::Call { callee, args } => eval_call(ctx, callee, args),
    }
}

fn eval_ident(ctx: &mut Ctx<'_>, name: &str) -> Result<Value, String> {
    if let Some(value) = ctx.lookup_var(name) {
        return Ok(value);
    }
    if ctx.in_eval && host::needs_prefix(name) {
        return Err(format!("{name} needs thisLayer inside eval"));
    }
    host::ident(ctx, name)
}

fn eval_call(ctx: &mut Ctx<'_>, callee: &Expr, args: &[Expr]) -> Result<Value, String> {
    if let Expr::Member { object, name } = callee {
        let object = eval_expr(ctx, object)?;
        let args = eval_args(ctx, args)?;
        return host::call_method(ctx, object, name, &args);
    }
    if let Expr::Ident(name) = callee {
        if let Some(Value::Function(function)) = ctx.lookup_var(name) {
            let args = eval_args(ctx, args)?;
            return call_function(ctx, &function, &args);
        }
        if let Ok(value) = eval_ident(ctx, name) {
            if matches!(
                value,
                Value::Layer { .. }
                    | Value::Group(_)
                    | Value::Property(_)
                    | Value::This
                    | Value::Comp(_)
                    | Value::Content { .. }
            ) {
                let args = eval_args(ctx, args)?;
                return host::call_value(ctx, value, &args);
            }
        }
        if ctx.in_eval && host::needs_prefix(name) {
            return Err(format!("{name} needs thisLayer inside eval"));
        }
        let args = eval_args(ctx, args)?;
        return host::call_global(ctx, name, &args);
    }
    let callee = eval_expr(ctx, callee)?;
    let args = eval_args(ctx, args)?;
    host::call_value(ctx, callee, &args)
}

fn eval_args(ctx: &mut Ctx<'_>, args: &[Expr]) -> Result<Vec<Value>, String> {
    let mut values = Vec::new();
    for arg in args {
        values.push(eval_unwrapped(ctx, arg)?);
    }
    Ok(values)
}

fn call_function(ctx: &mut Ctx<'_>, function: &Function, args: &[Value]) -> Result<Value, String> {
    ctx.scopes.push(BTreeMap::new());
    for (index, name) in function.params.iter().enumerate() {
        let value = args.get(index).cloned().unwrap_or(Value::Null);
        ctx.declare(name, value);
    }
    let flow = exec_stmts(ctx, &function.body);
    ctx.scopes.pop();
    Ok(flow?.value)
}

fn eval_binary(ctx: &mut Ctx<'_>, op: BinOp, left: &Expr, right: &Expr) -> Result<Value, String> {
    if matches!(op, BinOp::And | BinOp::Or) {
        let left = eval_unwrapped(ctx, left)?;
        let take_left =
            matches!(op, BinOp::And) && !left.truthy() || matches!(op, BinOp::Or) && left.truthy();
        if take_left {
            return Ok(left);
        }
        return eval_unwrapped(ctx, right);
    }
    let left = eval_unwrapped(ctx, left)?;
    let right = eval_unwrapped(ctx, right)?;
    if ctx.in_eval
        && matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div)
        && (matches!(left, Value::Array(_)) || matches!(right, Value::Array(_)))
    {
        return Err("array operator is not available inside eval".into());
    }
    host::binary(op, &left, &right)
}
