mod tests {
    use linkerloader::types::object::MAGIC_NUMBER;
    use linkerloader::utils::find_seg_start;

    #[test]
    fn test_magic_number() {
        assert_eq!(MAGIC_NUMBER, "LINK");
    }

    #[test]
    fn test_find_seg_start() {
        assert_eq!(find_seg_start(5, 3), 6);
        assert_eq!(find_seg_start(7, 3), 9);
        assert_eq!(find_seg_start(8, 4), 8);
        assert_eq!(find_seg_start(0x15B, 0x4), 0x15C);
        assert_eq!(find_seg_start(0x0, 0x5), 0x0);
        assert_eq!(find_seg_start(0x5, 0x0), 0x5);
        assert_eq!(find_seg_start(0x0, 0x0), 0x0);
        assert_eq!(find_seg_start(0x80, 0x10), 0x80);
        assert_eq!(find_seg_start(0x64, 0x10), 0x70);
    }
}
