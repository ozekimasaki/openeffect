use crate::confirmed::Confirmed;
use crate::light::SceneLightKind;
use crate::material::{AcceptShadows, CastShadows};
use crate::renderer::Renderer;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShadowOutcome {
    None,
    Cast,
    Unconfirmed,
}

pub const fn shadow_outcome(
    renderer: Renderer,
    kind: SceneLightKind,
    light_casts: bool,
    caster: CastShadows,
    receiver: AcceptShadows,
    in_light: bool,
    inside_shadow_volume: bool,
) -> ShadowOutcome {
    if !in_light || !caster_casts(caster) || !receiver_accepts(receiver) {
        return ShadowOutcome::None;
    }
    match kind {
        SceneLightKind::Ambient => ShadowOutcome::None,
        SceneLightKind::Environment => {
            if matches!(renderer, Renderer::Advanced3d) && light_casts && inside_shadow_volume {
                ShadowOutcome::Cast
            } else {
                ShadowOutcome::None
            }
        }
        SceneLightKind::Point => ShadowOutcome::Unconfirmed,
        SceneLightKind::Parallel | SceneLightKind::Spot => {
            if light_casts {
                ShadowOutcome::Cast
            } else {
                ShadowOutcome::None
            }
        }
    }
}

const fn caster_casts(mode: CastShadows) -> bool {
    match mode {
        CastShadows::Off => false,
        CastShadows::On | CastShadows::Only => true,
    }
}

const fn receiver_accepts(mode: AcceptShadows) -> bool {
    match mode {
        AcceptShadows::Off => false,
        AcceptShadows::On | AcceptShadows::Only => true,
    }
}

pub const fn shadow_controls_editable(
    kind: SceneLightKind,
    light_casts: bool,
    renderer: Renderer,
) -> Confirmed<bool> {
    match kind {
        SceneLightKind::Ambient => Confirmed::Known(false),
        SceneLightKind::Environment => {
            Confirmed::Known(matches!(renderer, Renderer::Advanced3d) && light_casts)
        }
        SceneLightKind::Point => Confirmed::Unconfirmed,
        SceneLightKind::Parallel | SceneLightKind::Spot => Confirmed::Known(light_casts),
    }
}

#[cfg(test)]
mod tests {
    use super::{shadow_controls_editable, shadow_outcome, Confirmed, ShadowOutcome};
    use crate::light::SceneLightKind;
    use crate::material::{AcceptShadows, CastShadows};
    use crate::renderer::Renderer;

    #[test]
    fn parallel_shadow_requires_light_caster_and_receiver() {
        assert_eq!(
            shadow_outcome(
                Renderer::Classic3d,
                SceneLightKind::Parallel,
                false,
                CastShadows::On,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::None
        );
        assert_eq!(
            shadow_outcome(
                Renderer::Classic3d,
                SceneLightKind::Parallel,
                true,
                CastShadows::Off,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::None
        );
        assert_eq!(
            shadow_outcome(
                Renderer::Classic3d,
                SceneLightKind::Parallel,
                true,
                CastShadows::On,
                AcceptShadows::Off,
                true,
                true
            ),
            ShadowOutcome::None
        );
        assert_eq!(
            shadow_outcome(
                Renderer::Advanced3d,
                SceneLightKind::Parallel,
                true,
                CastShadows::On,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::Cast
        );
        assert_eq!(
            shadow_controls_editable(SceneLightKind::Parallel, false, Renderer::Classic3d),
            Confirmed::Known(false)
        );
    }

    #[test]
    fn spot_shadow_stays_inside_the_cone() {
        assert_eq!(
            shadow_outcome(
                Renderer::Advanced3d,
                SceneLightKind::Spot,
                true,
                CastShadows::On,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::Cast
        );
        assert_eq!(
            shadow_outcome(
                Renderer::Advanced3d,
                SceneLightKind::Spot,
                true,
                CastShadows::On,
                AcceptShadows::On,
                false,
                true
            ),
            ShadowOutcome::None
        );
    }

    #[test]
    fn point_shadows_stay_unconfirmed() {
        assert_eq!(
            shadow_outcome(
                Renderer::Classic3d,
                SceneLightKind::Point,
                true,
                CastShadows::On,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::Unconfirmed
        );
        assert_eq!(
            shadow_outcome(
                Renderer::Advanced3d,
                SceneLightKind::Point,
                true,
                CastShadows::On,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::Unconfirmed
        );
        assert!(
            shadow_controls_editable(SceneLightKind::Point, true, Renderer::Classic3d)
                .is_unconfirmed()
        );
    }

    #[test]
    fn ambient_never_casts() {
        assert_eq!(
            shadow_outcome(
                Renderer::Classic3d,
                SceneLightKind::Ambient,
                true,
                CastShadows::On,
                AcceptShadows::On,
                true,
                true
            ),
            ShadowOutcome::None
        );
        assert_eq!(
            shadow_controls_editable(SceneLightKind::Ambient, true, Renderer::Classic3d),
            Confirmed::Known(false)
        );
    }
}
