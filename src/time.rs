use uefi::{
    prelude::{Boot, SystemTable},
    table::runtime::Time,
    Result as UefiResult,
};

pub fn get_time(st: &SystemTable<Boot>) -> UefiResult<Time> {
    st.runtime_services().get_time()
}
