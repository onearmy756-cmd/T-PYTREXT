//! [DEPRECATED] AI OS selector — sasa ipo `osselect.rs` (P2 yenye nguvu).
//! Wrapper hii inabaki kwa ulinganifu; pipeline inatumia `osselect::decide`.

pub async fn select_os(computer_specs: &str, user_need: &str) -> String {
    crate::osselect::decide(computer_specs, user_need).os
}
