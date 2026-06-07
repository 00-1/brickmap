// Throwaway viability check: can wgpu get a (possibly software) adapter headlessly?
fn main() {
    let instance = wgpu::Instance::default();
    for fallback in [false, true] {
        let a = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: fallback,
        }));
        match a {
            Ok(a) => println!("fallback={fallback}: OK {:?}", a.get_info()),
            Err(e) => println!("fallback={fallback}: NONE ({e:?})"),
        }
    }
}
