use std::io::{Read, Write};

fn main() {
    let mut binary_data = Vec::new();
    std::io::stdin().read_to_end(&mut binary_data).expect("failed to read stdin");

    // Umm... Nx16383?
    let width = (binary_data.len() as u32).div_ceil(16383);
    let height = (binary_data.len() as u32).div_ceil(width);
    eprintln!("{width}x{height}");

    // Convert to grayscale RGB, padding the image to width * height
    let mut image_data: Vec<u8> = binary_data.iter().flat_map(|&b| [b, b, b]).collect();
    image_data.resize((width * height * 3) as usize, 0);

    // Lossless, quality 100, method 5
    let mut config = webp::WebPConfig::new().unwrap();
    config.lossless = 1;
    config.quality = 100.0;
    config.method = 5;
    let compressed = webp::Encoder::from_rgb(&image_data, width, height)
        .encode_advanced(&config)
        .expect("encoding failed");

    std::io::stdout().write_all(&compressed).expect("failed to write to stdout");
}
