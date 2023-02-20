pub async fn process() {
    crate::print_fb!("\0");
    while let Some(s) = crate::kbuf::read().await {
        crate::print_fb!("{}", s);
    }
}
