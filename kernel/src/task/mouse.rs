super::stream_processor_task!(u8, 1024);

pub async fn process() {
    let mouse = crate::peripheral::mouse::get().expect("mouse should be initialized by now");
    let mut stream = TaskStream::new();
    while let Some(packet) = stream.next().await {
        mouse.add(packet).await;
    }
}
