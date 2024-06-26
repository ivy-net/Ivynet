/// Note on nomenclature: We use snake_case module names instead of properly scoped names
/// (ivy_daemon.types) because the latter does not work with tonic::include_proto!(), which breaks
/// on the period in the string and does not handle module renaming.

pub mod ivy_daemon {
    tonic::include_proto!("ivy_daemon");
}

pub mod ivy_daemon_types {
    tonic::include_proto!("ivy_daemon_types");
}

pub mod ivy_daemon_avs {
    tonic::include_proto!("ivy_daemon_avs");
}

#[test]
fn test_api_types() {}
