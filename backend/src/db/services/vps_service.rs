// This file now acts as a re-exporter for the vps-related services.

// Re-export structs and functions from the core VPS operations module
pub use super::vps_core_service::{
    create_vps, get_all_vps_for_user, get_vps_by_id, get_vps_by_user_id, update_vps,
    update_vps_info_on_handshake, update_vps_status,
};

// Re-export structs and functions from the VPS renewal operations module
pub use super::vps_renewal_service::{
    check_and_generate_reminders, create_or_update_vps_renewal_info,
    dismiss_vps_renewal_reminder, get_vps_renewal_info_by_vps_id,
    process_all_automatic_renewals, VpsRenewalDataInput,
};

// Re-export functions from the VPS traffic operations module
pub use super::vps_traffic_service::{
    get_vps_due_for_traffic_reset, process_vps_traffic_reset,
    update_vps_traffic_stats_after_metric,
};

// Re-export functions from the VPS detail retrieval module
pub use super::vps_detail_service::{
    get_all_vps_with_details_for_cache, get_all_vps_with_details_for_user,
    get_vps_with_details_for_cache_by_id,
};

// Common models that might be part of the public API of the functions above
// are typically defined in `crate::db::models` or `crate::websocket_models`
// and imported directly by the consumer of these service functions or by the
// individual service modules themselves.
// For example, `Vps` from `crate::db::models::Vps` or
// `ServerWithDetails` from `crate::websocket_models::ServerWithDetails`.
// `VpsRenewalDataInput` is an exception as it's defined within `vps_renewal_service`
// and re-exported here for convenience if `vps_service::VpsRenewalDataInput` is preferred.