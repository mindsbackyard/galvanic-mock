macro_rules! acquire {
    ( $global_var: ident ) => { $global_var.lock().unwrap() }
}
