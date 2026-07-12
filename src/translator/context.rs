use crate::engine::data_sources::CsvDataSource;
use crate::engine::reporting::RealTimeMetrics;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[cfg(feature = "grpc")]
use crate::engine::grpc_dynamic::DynamicGrpcClient;

/// Type alias for the gRPC client map. When the `grpc` feature is disabled,
/// this is a zero-sized type that avoids pulling in tonic/prost dependencies.
#[cfg(feature = "grpc")]
pub(crate) type GrpcClients = Arc<HashMap<String, Arc<DynamicGrpcClient>>>;
#[cfg(not(feature = "grpc"))]
pub(crate) type GrpcClients = ();

/// Shared context for transaction construction.
///
/// Groups scenario-level configuration and shared state that every transaction
/// closure needs. All fields are `Arc`-wrapped so cloning the context is an
/// O(1) atomic reference-count increment instead of deep-copying the data.
#[derive(Clone)]
pub(crate) struct TransactionContext {
    pub scenario_name: Arc<String>,
    pub scenario_headers: Arc<Option<HashMap<String, String>>>,
    pub data_sources: Arc<HashMap<String, Vec<Arc<CsvDataSource>>>>,
    pub real_time_metrics: Option<Arc<RwLock<RealTimeMetrics>>>,
    #[allow(dead_code)]
    pub grpc_clients: GrpcClients,
}
