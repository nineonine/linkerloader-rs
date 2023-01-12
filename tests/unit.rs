mod tests {
    use linkerloader::types::object::MAGIC_NUMBER;

    #[test]
    fn magic_number() {
        assert_eq!(MAGIC_NUMBER, "LINK");
    }
}
