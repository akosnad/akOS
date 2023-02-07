super::stream_processor_task!(u8, 1024);

pub async fn process() {
    let keyboard =
        crate::peripheral::keyboard::get().expect("keyboard should be initialized by now");
    let mut stream = TaskStream::new();
    while let Some(sc) = stream.next().await {
        keyboard.add(sc).await;
    }
}
