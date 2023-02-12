mod tests {
    use linkerloader::types::object::MAGIC_NUMBER;
    use linkerloader::utils::{find_seg_start, mk_addr_4};

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

    #[test]
    fn test_mk_addr_4() {
        assert!(mk_addr_4(-1).is_none());
        assert!(mk_addr_4(65536).is_none());

        let mut v1 = mk_addr_4(65535);
        assert!(v1.is_some());
        assert_eq!(255, v1.as_ref().unwrap()[0]);
        assert_eq!(255, v1.as_ref().unwrap()[1]);

        v1 = mk_addr_4(0);
        assert!(v1.is_some());
        assert_eq!(0, v1.as_ref().unwrap()[0]);
        assert_eq!(0, v1.as_ref().unwrap()[1]);

        v1 = mk_addr_4(43775);
        assert!(v1.is_some());
        assert_eq!(170, v1.as_ref().unwrap()[0]);
        assert_eq!(255, v1.as_ref().unwrap()[1]);

        v1 = mk_addr_4(511);
        assert!(v1.is_some());
        assert_eq!(1, v1.as_ref().unwrap()[0]);
        assert_eq!(255, v1.as_ref().unwrap()[1]);
    }
}
