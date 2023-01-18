use rand::Rng;
// helper function for generating random object data that is used in tests
pub fn gen_obj_data(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| format!("{:X}{:X}", rng.gen_range(0..16), rng.gen_range(0..16)))
        .collect::<Vec<String>>()
        .join(" ")
}
