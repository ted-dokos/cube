use crate::texture;

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    let path = std::path::Path::new(env!("OUT_DIR")).join("assets").join(file_name);
    let data = std::fs::read(path)?;

    Ok(data)
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    let path = std::path::Path::new(env!("OUT_DIR")).join("assets").join(file_name);
    let txt = std::fs::read_to_string(path)?;

    Ok(txt)
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;

    texture::Texture::from_bytes(device, queue, &data, file_name)
}
