/// Windows では DX12 だけを有効にする。
///
/// `Backends::PRIMARY` は Vulkan も含むため、Windows では使わない。
/// ここはインスタンスの設定だけを返す。アダプタの作成とウィンドウは行わない。
pub fn preferred_backends() -> wgpu::Backends {
    #[cfg(target_os = "windows")]
    {
        return wgpu::Backends::DX12;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut backends = wgpu::Backends::PRIMARY;
        backends.remove(wgpu::Backends::DX12);
        backends
    }
}

pub fn instance_descriptor() -> wgpu::InstanceDescriptor {
    wgpu::InstanceDescriptor {
        backends: preferred_backends(),
        ..wgpu::InstanceDescriptor::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{instance_descriptor, preferred_backends};

    #[test]
    fn selects_dx12_on_windows_without_opening_a_device() {
        let backends = preferred_backends();
        if cfg!(target_os = "windows") {
            assert_eq!(backends, wgpu::Backends::DX12);
        } else {
            assert!(!backends.contains(wgpu::Backends::DX12));
        }
        assert_eq!(instance_descriptor().backends, backends);
    }
}
