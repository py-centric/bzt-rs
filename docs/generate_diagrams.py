import os
from diagrams import Cluster, Diagram, Edge
from diagrams.programming.language import Rust
from diagrams.onprem.client import User
from diagrams.onprem.network import Nginx
from diagrams.onprem.database import PostgreSQL, InfluxDB
from diagrams.onprem.inmemory import Redis
from diagrams.onprem.queue import Kafka
from diagrams.onprem.compute import Server
from diagrams.onprem.security import Vault
from diagrams.onprem.container import Docker
from diagrams.k8s import K8S as K8s
from diagrams.aws.compute import EKS, Lambda, EC2
from diagrams.aws.database import RDS
from diagrams.aws.analytics import ManagedStreamingForKafka as MSK
from diagrams.aws.storage import S3
from diagrams.aws.network import ELB
from diagrams.gcp.compute import GKE, Functions as GCF, ComputeEngine as GCE
from diagrams.gcp.database import SQL as CloudSQL
from diagrams.gcp.analytics import Pubsub
from diagrams.gcp.storage import GCS
from diagrams.gcp.network import LoadBalancing as GCPLB
from diagrams.azure.compute import AKS, FunctionApps, VM
from diagrams.azure.database import CosmosDb, SQLDatabases as AzureSQL
from diagrams.azure.analytics import EventHubs
from diagrams.azure.storage import BlobStorage
from diagrams.azure.network import LoadBalancers as AzureLB
from diagrams.onprem.monitoring import Prometheus
from diagrams.onprem.ci import GithubActions

def create_folders():
    """Create the required output folder structure using absolute paths."""
    base_output = os.path.abspath("docs/diagrams")
    subfolders = [
        "01_Core_Architecture",
        "02_Use_Cases",
        "03_User_Journeys",
        "04_Security",
        "05_Cloud_Deployments",
        "06_Deployment_Strategies"
    ]
    for sub in subfolders:
        path = os.path.join(base_output, sub)
        os.makedirs(path, exist_ok=True)
    return base_output

# ==========================================
# 01. CORE ARCHITECTURE
# ==========================================
def generate_master_interaction_map(output_path):
    graph_attr = {"nodesep": "0.8", "ranksep": "0.8"}
    with Diagram("pummel Architecture Interaction Map", 
                 filename=os.path.join(output_path, "01_Core_Architecture/master_map"),
                 show=False, direction="TB", graph_attr=graph_attr):
        user = User("Performance Engineer")
        with Cluster("pummel CLI"):
            cli = Rust("CLI (main.rs)")
            parser = Server("Multi-Format Parser")
            normalizer = Server("Schema Normalizer")
            validation = Server("Async Validation")
        with Cluster("Execution Core"):
            discovery = Server("gRPC Service Discovery")
            translator = Server("Async State Translator")
            mock = Server("Multi-Protocol Mock")
            goose = Rust("Goose Load Engine")
            services = Server("Shell Hook Executor\n(Chaos Setup/Cleanup)")
            api = Server("Dynamic Control API\n(Metrics & Stop)")
            sla_rt = Server("Real-Time SLA Checker")
        with Cluster("Observability"):
            influx = InfluxDB("Real-time InfluxDB")
            metrics = Prometheus("Aggregated Metrics")
            junit = Server("JUnit XML")
            html_report = Server("HTML Report")
        
        with Cluster("Targets"):
            rest = Server("REST / HTTP")
            ws = Server("WebSockets")
            grpc = Server("Dynamic gRPC")

        user >> cli >> parser >> normalizer >> validation
        validation >> discovery >> translator >> goose
        goose >> Edge(color="red") >> rest
        goose >> Edge(color="blue") >> ws
        goose >> Edge(color="green") >> grpc
        
        validation >> Edge(color="orange", style="dashed") >> mock
        
        # Chaos & Control flows
        cli >> services >> goose
        goose >> Edge(color="purple", style="dashed") >> sla_rt >> Edge(color="red", style="dashed", label="Breach Exec") >> services
        api >> Edge(color="darkgreen", label="Query / Stop") >> goose
        
        goose >> influx
        goose >> metrics
        goose >> junit
        goose >> html_report

# ==========================================
# 02. DETAILED SCENARIOS
# ==========================================

def generate_sc_dry_run_logic(output_path):
    """Detailed Scenario: How Dry-Run works (Fixed spacing and direction)."""
    graph_attr = {"nodesep": "1.0", "ranksep": "1.0"}
    with Diagram("Scenario: Configuration Dry-Run Validation", 
                 filename=os.path.join(output_path, "02_Use_Cases/sc_dry_run_detail"),
                 show=False, direction="TB", graph_attr=graph_attr):
        
        user = User("User (pummel --dry-run)")
        
        with Cluster("Validation Pipeline"):
            syntax = Server("1. Syntax Check\n(YAML/JSON/TOML)")
            schema = Server("2. Taurus Schema\nAlignment")
            resources = Server("3. Resource Integrity\n(CSV/Data Sources)")
            env_vars = Server("4. Environment Variable\nSubstitution")
            
        success = Server("Report: PASS\n(Exit 0)")
        failure = Server("Report: FAIL\n(Exit 1)")

        user >> syntax >> schema >> resources >> env_vars
        env_vars >> Edge(color="green", label="Validated") >> success
        env_vars >> Edge(color="red", label="Error Found") >> failure

def generate_sc_branching_logic(output_path):
    """Scenario: Weighted branching and hierarchical scenarios."""
    with Diagram("Scenario: Weighted Branching & Hierarchy", 
                 filename=os.path.join(output_path, "02_Use_Cases/sc_branching_logic"),
                 show=False, direction="LR"):
        
        engine = Rust("Goose Engine")
        
        with Cluster("Scenario Hierarchy"):
            parent = Server("Parent: Authenticate (100%)")
            with Cluster("Weighted Children"):
                b1 = Server("Child A: Search (80%)")
                b2 = Server("Child B: Checkout (20%)")
                
        engine >> Edge(label="Executes Once") >> parent
        parent >> Edge(label="Branch 0.8") >> b1
        parent >> Edge(label="Branch 0.2") >> b2
        
        target = Server("Target API")
        b1 >> target
        b2 >> target

def generate_sc_session_state(output_path):
    """Scenario: Complex Session State & Variable Lifecycle."""
    with Diagram("Scenario: Complex Session State Lifecycle", 
                 filename=os.path.join(output_path, "02_Use_Cases/sc_session_state"),
                 show=False, direction="LR"):
        
        user_session = Server("Virtual User Session")
        
        with Cluster("Step 1: Handshake"):
            req1 = Server("GET /auth")
            ext1 = Server("Extract: Cookie/Token")
            
        with Cluster("Step 2: Processing"):
            req2 = Server("POST /data")
            ext2 = Server("Extract: EntityID")
            
        with Cluster("Step 3: Cleanup"):
            req3 = Server("DELETE /data/${EntityID}")

        user_session >> req1 >> ext1
        ext1 >> Edge(label="Store: Token") >> user_session
        user_session >> Edge(label="Inject: Token") >> req2 >> ext2
        ext2 >> Edge(label="Store: EntityID") >> user_session
        user_session >> Edge(label="Inject: EntityID") >> req3

def generate_sc_grpc_discovery(output_path):
    """Scenario: Dynamic gRPC Reflection & Discovery."""
    graph_attr = {"nodesep": "0.8", "ranksep": "0.8"}
    with Diagram("Scenario: Dynamic gRPC Discovery", 
                 filename=os.path.join(output_path, "02_Use_Cases/sc_grpc_discovery"),
                 show=False, direction="LR", graph_attr=graph_attr):
        
        translator = Server("Async Translator")
        
        with Cluster("Reflection Phase"):
            client = Server("Reflection Client")
            target = Server("gRPC Server\n(Reflection Enabled)")
            pool = Server("Descriptor Pool")
            
        with Cluster("Execution Phase"):
            dynamic_msg = Server("Dynamic Message\n(JSON <-> Proto)")
            call = Server("Generic gRPC Call")

        translator >> client >> Edge(label="ListServices") >> target
        target >> Edge(label="FileDescriptors") >> client >> pool
        pool >> dynamic_msg >> call >> target

def generate_sc_realtime_observability(output_path):
    """Scenario: Real-time InfluxDB reporting lifecycle."""
    with Diagram("Scenario: Real-time Observability", 
                 filename=os.path.join(output_path, "02_Use_Cases/sc_realtime_obs"),
                 show=False, direction="TB"):
        
        with Cluster("Worker Node (UUID: worker-123)"):
            goose = Rust("Goose Engine")
            shared_state = Server("Shared RealTimeMetrics\n(Arc<Mutex>)")
            bg_task = Server("Background Reporting Task")
            
        influx = InfluxDB("InfluxDB")
        grafana = Server("Grafana Dashboard")

        goose >> Edge(label="Update Delta") >> shared_state
        bg_task >> Edge(label="Poll every 10s") >> shared_state
        bg_task >> Edge(label="Push with worker_id") >> influx
        influx >> grafana

def generate_sc_chaos_engineering(output_path):
    """Scenario: Chaos Engineering Lifecycle with hooks, SLA breach actions, and control API."""
    graph_attr = {"nodesep": "0.8", "ranksep": "0.8"}
    with Diagram("Scenario: Chaos Engineering Lifecycle",
                 filename=os.path.join(output_path, "02_Use_Cases/sc_chaos_engineering"),
                 show=False, direction="TB", graph_attr=graph_attr):

        user = User("Performance Engineer")

        with Cluster("Chaos Lifecycle"):
            prepare = Server("Shell Hooks\n(prepare)")
            startup = Server("Shell Hooks\n(startup)")
            shutdown = Server("Shell Hooks\n(shutdown)")

        with Cluster("Execution Core"):
            goose = Rust("Goose Engine")
            sla = Server("Real-Time SLA Checker")
            api = Server("Control API")

        target = Server("Target System")

        user >> prepare >> startup >> goose
        goose >> Edge(color="red") >> target
        goose >> Edge(color="purple", style="dashed", label="Metrics") >> sla
        sla >> Edge(color="red", style="dashed", label="Breach Exec") >> startup
        api >> Edge(color="darkgreen", label="Query / Stop") >> goose
        goose >> Edge(color="grey", style="dotted", label="Finishes") >> shutdown

# ==========================================
# 05/06. CLOUD DEPLOYMENTS & STRATEGIES
# ==========================================

def generate_multi_cloud_topologies(output_path):
    # AWS Topology
    with Diagram("Deployment: AWS Distributed Topology", 
                 filename=os.path.join(output_path, "05_Cloud_Deployments/aws_topology"),
                 show=False):
        eks = EKS("pummel Controller (EKS)")
        workers = [EKS("Worker 1"), EKS("Worker 2")]
        target = Server("Target AWS App")
        eks >> workers >> target
        workers >> Edge(color="orange") >> MSK("Metrics (MSK)")

    # GCP Topology
    with Diagram("Deployment: GCP Distributed Topology", 
                 filename=os.path.join(output_path, "05_Cloud_Deployments/gcp_topology"),
                 show=False):
        gke = GKE("pummel Controller (GKE)")
        workers = [GKE("Worker 1"), GKE("Worker 2")]
        target = Server("Target GCP App")
        gke >> workers >> target
        workers >> Edge(color="orange") >> Pubsub("Metrics (Pub/Sub)")

    # Azure Topology
    with Diagram("Deployment: Azure Distributed Topology", 
                 filename=os.path.join(output_path, "05_Cloud_Deployments/azure_topology"),
                 show=False):
        aks = AKS("pummel Controller (AKS)")
        workers = [AKS("Worker 1"), AKS("Worker 2")]
        target = Server("Target Azure App")
        aks >> workers >> target
        workers >> Edge(color="orange") >> EventHubs("Telemetry")

def generate_deployment_aws_strategies(output_path):
    """AWS Deployment Strategies: Lambda vs EC2."""
    with Diagram("Strategy: Serverless Load (AWS Lambda)", 
                 filename=os.path.join(output_path, "06_Deployment_Strategies/aws_lambda_strategy"),
                 show=False):
        user = User("DevOps")
        with Cluster("AWS Cloud"):
            invoker = Lambda("pummel Invoker")
            lambdas = [Lambda("Worker 1"), Lambda("Worker 2"), Lambda("Worker 3")]
            target = Server("Target System")
            user >> invoker >> lambdas >> Edge(color="red") >> target

    with Diagram("Strategy: Sustained Load (AWS EC2)", 
                 filename=os.path.join(output_path, "06_Deployment_Strategies/aws_ec2_strategy"),
                 show=False):
        user = User("SRE")
        with Cluster("AWS VPC"):
            lb = ELB("Load Balancer")
            nodes = [EC2("Node 1"), EC2("Node 2")]
            target = Server("High-Load Target")
            user >> lb >> nodes >> Edge(color="red") >> target

def generate_deployment_gcp_strategies(output_path):
    """GCP Deployment Strategies: Cloud Functions vs Compute Engine."""
    with Diagram("Strategy: Serverless Load (GCP Cloud Functions)", 
                 filename=os.path.join(output_path, "06_Deployment_Strategies/gcp_functions_strategy"),
                 show=False):
        user = User("DevOps")
        with Cluster("GCP Project"):
            invoker = GCF("pummel Invoker")
            functions = [GCF("Worker 1"), GCF("Worker 2"), GCF("Worker 3")]
            target = Server("Target System")
            user >> invoker >> functions >> Edge(color="red") >> target

    with Diagram("Strategy: Sustained Load (GCP Compute Engine)", 
                 filename=os.path.join(output_path, "06_Deployment_Strategies/gcp_compute_strategy"),
                 show=False):
        user = User("SRE")
        with Cluster("GCP VPC"):
            lb = GCPLB("Load Balancer")
            nodes = [GCE("GCE Instance 1"), GCE("GCE Instance 2")]
            target = Server("High-Load Target")
            user >> lb >> nodes >> Edge(color="red") >> target

def generate_deployment_azure_strategies(output_path):
    """Azure Deployment Strategies: Azure Functions vs VMs."""
    with Diagram("Strategy: Serverless Load (Azure Functions)", 
                 filename=os.path.join(output_path, "06_Deployment_Strategies/azure_functions_strategy"),
                 show=False):
        user = User("DevOps")
        with Cluster("Azure Subscription"):
            invoker = FunctionApps("pummel Invoker")
            functions = [FunctionApps("Worker 1"), FunctionApps("Worker 2"), FunctionApps("Worker 3")]
            target = Server("Target System")
            user >> invoker >> functions >> Edge(color="red") >> target

    with Diagram("Strategy: Sustained Load (Azure VMs)", 
                 filename=os.path.join(output_path, "06_Deployment_Strategies/azure_vm_strategy"),
                 show=False):
        user = User("SRE")
        with Cluster("Azure Virtual Network"):
            lb = AzureLB("Load Balancer")
            nodes = [VM("Azure VM 1"), VM("Azure VM 2")]
            target = Server("High-Load Target")
            user >> lb >> nodes >> Edge(color="red") >> target

if __name__ == "__main__":
    print("Starting pummel exhaustive diagram generation...")
    output_dir = create_folders()
    print(f"Output directory initialized at: {output_dir}")
    
    try:
        generate_master_interaction_map(output_dir)
        generate_sc_dry_run_logic(output_dir)
        generate_sc_branching_logic(output_dir)
        generate_sc_session_state(output_dir)
        generate_sc_grpc_discovery(output_dir)
        generate_sc_realtime_observability(output_dir)
        generate_sc_chaos_engineering(output_dir)
        generate_multi_cloud_topologies(output_dir)
        generate_deployment_aws_strategies(output_dir)
        generate_deployment_gcp_strategies(output_dir)
        generate_deployment_azure_strategies(output_dir)
        
        # Simple verification output
        print("\nVerification of generated files:")
        for root, dirs, files in os.walk(output_dir):
            for file in files:
                if file.endswith(".png"):
                    print(f"  [CREATED] {os.path.join(root, file)}")
        
        print(f"\nSuccess! pummel diagrams updated in '{output_dir}'.")
    except Exception as e:
        print(f"Error generating diagrams: {e}")
