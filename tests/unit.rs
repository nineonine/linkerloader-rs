
#[cfg(test)]
mod tests {
    use linkerloader::lib::MAGIC_NUMBER;
    #[test]
    fn magic_number() {
        assert_eq!(MAGIC_NUMBER, "LINK");
    }
}
