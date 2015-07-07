// core.h

#[link(name = "sodium", kind = "static")]
extern {
    pub fn sodium_init() -> c_int;
}
