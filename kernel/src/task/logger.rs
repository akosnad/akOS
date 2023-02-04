pub async fn process() {
    while let Some(s) = crate::kbuf::read().await {
        crate::print_fb!("{}", s);
    }
}
