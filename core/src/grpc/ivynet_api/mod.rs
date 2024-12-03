/// Note on nomenclature: We use snake_case module names instead of properly scoped names
/// (ivy_daemon.types) because the latter does not work with tonic::include_proto!(), which breaks
/// on the period in the string and does not handle module renaming.

pub mod ivy_daemon_operator {
    tonic::include_proto!("operator");
}
