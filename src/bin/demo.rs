fn main() {
    let path = std::env::args().nth(1).unwrap();
    let frames = paranormal::process(image::open(path).unwrap().to_rgb());

    let mut file = std::fs::File::create("out.png").unwrap();
    let mut encoder = apng_encoder::Encoder::create(
        &mut file,
        apng_encoder::Meta {
            width: frames[0].width(),
            height: frames[0].height(),
            color: apng_encoder::Color::RGB(8),
            frames: frames.len() as u32,
            plays: None,
        },
    )
    .unwrap();
    for frame in frames {
        encoder
            .write_frame(
                &frame.into_raw(),
                Some(&apng_encoder::Frame {
                    delay: Some(apng_encoder::Delay::new(1, 10)),
                    ..Default::default()
                }),
                None,
                None,
            )
            .unwrap();
    }
    encoder.finish().unwrap();
}
