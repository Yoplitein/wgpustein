#![allow(dead_code, unused_imports, unsafe_op_in_unsafe_fn)]

unsafe extern "C" {
	fn clog(x: i32);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn startmepls() {
	clog(69);
}
