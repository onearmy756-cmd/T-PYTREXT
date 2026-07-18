from .core import (
    PyTreXApp, event, execute_python_event, execute_python_event_async,
    ElixirClient, BLOCKCHAIN_CACHE, REGISTERED_EVENTS,
    AuthManager, InputValidator, SecureKeyStorage, OfflineSyncQueue,
    PluginManager, I18n, MobileAPI,
    BiometricAuth, PushNotifications, QRCodeManager, SystemTray,
    DeepLinking, APIServer, CrashReporter, Analytics,
    PDFGenerator, Compression, ImageProcessor, BackgroundService,
    WebSocketServer, CronScheduler, EmailService, PDFViewer,
    ChartVisualizer, MediaPlayer, FileWatcher, ClipboardManager,
    ScreenshotCapture, NetworkScanner, ConfigManager, SessionManager,
    TerminalEmulator, CodeEditor, DatabaseMigrations, GraphQLServer,
    OAuth2Integration, WebRTCVideoCall, BarcodeScanner, GeolocationMaps,
    BluetoothManager, USBDeviceManager, ProcessManager, ThemeManager,
    AutoFixEngine, HealthChecker, EncryptionManager, CacheManager,
    TaskQueue, NotificationManager, BackupManager, LogManager,
    DependencyChecker, PerformanceMonitor,
    StateMachine, EventBus, ValidatorEngine, Localization,
    FeatureFlags, RateLimiter, RetryEngine, CircuitBreaker,
    SecretVault, APIClient,
    WSClient, RedisPubSub, SearchEngine, MLInference,
    DataExporter, DataImporter, JobScheduler, WebScraper,
    PDFGeneratorPro, SMSGateway, PaymentGateway, AIChatAssistant,
    LLMIntegration, VectorDatabase, AIAgent, EmbeddingEngine,
    TextSummarizer, SentimentAnalyzer, LanguageDetector, ImageClassifier,
    SpeechToText, TextToSpeech, CodeGenerator, RAGEngine,
    ORMEngine, ORMQuery, WorkflowEngine, TemplateEngine,
    FormBuilder, MessageQueue, StreamProcessor, TimeSeriesDB,
    GraphDatabase, DocGenerator, TestFramework, CLIBuilder, IoTManager,
    RealtimeSync, PermissionsEngine, AuditTrail, MultiTenantManager,
    WebhookManager, VersionControl, ABTesting, FeatureAnalytics,
    ContentModerator, RecommendationEngine, DataPipeline, ServiceMesh,
    SecurityScanner, SmartContract, StatisticsEngine, CICDPipeline,
    NetworkTools, OCREngine, CloudManager, GameEngine,
    QuantumSimulator, SQLBuilder, TranslationEngine, EdgeCompute,
    ModelTrainer, PredictiveAnalytics, AnomalyDetector, NLPProcessor,
    ImageGenerator, VoiceCloner, AutoML, FederatedLearning,
    ModelRegistry, DataLabeler, PromptEngine, ChatbotFramework,
    SSOManager, LDAPManager, SAMLProvider, AuditReporter,
    ComplianceChecker, DataGovernance, PrivacyManager, GDPRTools,
    EncryptionVault, KeyRotation, AccessPolicy, IdentityProvider,
    CodeFormatter, Linter, TypeChecker, DebugProfiler,
    MemoryAnalyzer, HotReloader, REPL, Notebook,
    APITester, MockServer, SnapshotTest, CoverageReporter,
    HealthcareHL7, FinancePortfolio, InventorySCM, HRPayroll,
    CRMPipeline, ProjectKanban, InvoiceGenerator, TaxCalculator,
    GeoGIS, IoTProtocol, EnergyGrid, LogisticsRoute,
    NeuralNetwork, ConvolutionalNN, RecurrentNN, TransformerModel,
    GANEngine, ReinforcementLearning, Optimizer, LossFunctions,
    ActivationFunctions, Regularization, AttentionMechanism,
    TransferLearning, ModelCheckpoint, HyperparameterTuner,
    ConfusionMatrix, DataAugmentation,
    BlockchainBridge,
    # === PyTreXT Extended: New Modules ===
    AxumBridge, CandleBridge, BurnBridge, MCPBridge,
)

# === PyTreXT Extended: New Module Imports ===
from .langchain_agent import (
    LangChainAgent, ToolDefinition, create_pytrex_langchain_tools,
)
from .search_engine import (
    SearchEngine as WebSearchEngine, SearchResult,
    quick_search, quick_web_summary,
)
from .human_in_loop import (
    HumanInTheLoop, ActionStatus, PendingAction, create_hitl_workflow,
)
from .hermes_agent import (
    HermesAgent, FunctionDefinition, FunctionCall, ToolChoice,
)
from .mcp_client import (
    MCPClient, MCPTool, MCPResource, MCPPrompt, MCPTransport,
    connect_to_pytrex_mcp, discover_mcp_tools,
)

# === PyTreXT Extended: TestRunner ===
from .test_runner import (
    TestRunner, TestSuite, TestResult, TestStatus,
    test_app, quick_test, test_module,
)

# === PyTreXT Extended: Project Manager ===
from .project_manager import (
    ProjectManager, Project,
)

# === PyTreXT Extended: Production Builder ===
from .production import (
    ProductionBuilder, ProductionConfig, DeployTarget,
    deploy, quick_build,
)

__all__ = [
    "PyTreXApp", "event", "ElixirClient", "BLOCKCHAIN_CACHE",
    "execute_python_event", "execute_python_event_async",
    "REGISTERED_EVENTS", "AuthManager", "InputValidator",
    "SecureKeyStorage", "OfflineSyncQueue", "PluginManager", "I18n",
    "MobileAPI", "BiometricAuth", "PushNotifications", "QRCodeManager",
    "SystemTray", "DeepLinking", "APIServer", "CrashReporter",
    "Analytics", "PDFGenerator", "Compression", "ImageProcessor",
    "BackgroundService", "WebSocketServer", "CronScheduler",
    "EmailService", "PDFViewer", "ChartVisualizer", "MediaPlayer",
    "FileWatcher", "ClipboardManager", "ScreenshotCapture",
    "NetworkScanner", "ConfigManager", "SessionManager",
    "TerminalEmulator", "CodeEditor", "DatabaseMigrations",
    "GraphQLServer", "OAuth2Integration", "WebRTCVideoCall",
    "BarcodeScanner", "GeolocationMaps", "BluetoothManager",
    "USBDeviceManager", "ProcessManager", "ThemeManager",
    "AutoFixEngine", "HealthChecker", "EncryptionManager", "CacheManager",
    "TaskQueue", "NotificationManager", "BackupManager", "LogManager",
    "DependencyChecker", "PerformanceMonitor",
    "StateMachine", "EventBus", "ValidatorEngine", "Localization",
    "FeatureFlags", "RateLimiter", "RetryEngine", "CircuitBreaker",
    "SecretVault", "APIClient",
    "WSClient", "RedisPubSub", "SearchEngine", "MLInference",
    "DataExporter", "DataImporter", "JobScheduler", "WebScraper",
    "PDFGeneratorPro", "SMSGateway", "PaymentGateway", "AIChatAssistant",
    "LLMIntegration", "VectorDatabase", "AIAgent", "EmbeddingEngine",
    "TextSummarizer", "SentimentAnalyzer", "LanguageDetector", "ImageClassifier",
    "SpeechToText", "TextToSpeech", "CodeGenerator", "RAGEngine",
    "ORMEngine", "ORMQuery", "WorkflowEngine", "TemplateEngine",
    "FormBuilder", "MessageQueue", "StreamProcessor", "TimeSeriesDB",
    "GraphDatabase", "DocGenerator", "TestFramework", "CLIBuilder", "IoTManager",
    "RealtimeSync", "PermissionsEngine", "AuditTrail", "MultiTenantManager",
    "WebhookManager", "VersionControl", "ABTesting", "FeatureAnalytics",
    "ContentModerator", "RecommendationEngine", "DataPipeline", "ServiceMesh",
    "SecurityScanner", "SmartContract", "StatisticsEngine", "CICDPipeline",
    "NetworkTools", "OCREngine", "CloudManager", "GameEngine",
    "QuantumSimulator", "SQLBuilder", "TranslationEngine", "EdgeCompute",
    "ModelTrainer", "PredictiveAnalytics", "AnomalyDetector", "NLPProcessor",
    "ImageGenerator", "VoiceCloner", "AutoML", "FederatedLearning",
    "ModelRegistry", "DataLabeler", "PromptEngine", "ChatbotFramework",
    "SSOManager", "LDAPManager", "SAMLProvider", "AuditReporter",
    "ComplianceChecker", "DataGovernance", "PrivacyManager", "GDPRTools",
    "EncryptionVault", "KeyRotation", "AccessPolicy", "IdentityProvider",
    "CodeFormatter", "Linter", "TypeChecker", "DebugProfiler",
    "MemoryAnalyzer", "HotReloader", "REPL", "Notebook",
    "APITester", "MockServer", "SnapshotTest", "CoverageReporter",
    "HealthcareHL7", "FinancePortfolio", "InventorySCM", "HRPayroll",
    "CRMPipeline", "ProjectKanban", "InvoiceGenerator", "TaxCalculator",
    "GeoGIS", "IoTProtocol", "EnergyGrid", "LogisticsRoute",
    "NeuralNetwork", "ConvolutionalNN", "RecurrentNN", "TransformerModel",
    "GANEngine", "ReinforcementLearning", "Optimizer", "LossFunctions",
    "ActivationFunctions", "Regularization", "AttentionMechanism",
    "TransferLearning", "ModelCheckpoint", "HyperparameterTuner",
    "ConfusionMatrix", "DataAugmentation",
    "BlockchainBridge",
    # === PyTreXT Extended ===
    "AxumBridge", "CandleBridge", "BurnBridge", "MCPBridge",
    "LangChainAgent", "ToolDefinition", "create_pytrex_langchain_tools",
    "WebSearchEngine", "SearchResult", "quick_search", "quick_web_summary",
    "HumanInTheLoop", "ActionStatus", "PendngAction", "create_hitl_workflow",
    "HermesAgent", "FunctionDefinition", "FunctionCall", "ToolChoice",
    "MCPClient", "MCPTool", "MCPResource", "MCPPrompt", "MCPTransport",
    "connect_to_pytrex_mcp", "discover_mcp_tools",
    # === PyTreXT Extended: TestRunner ===
    "TestRunner", "TestSuite", "TestResult", "TestStatus",
    "test_app", "quick_test", "test_module",
    # === PyTreXT Extended: Project Manager ===
    "ProjectManager", "Project",
    # === PyTreXT Extended: Production Builder ===
    "ProductionBuilder", "ProductionConfig", "DeployTarget",
    "deploy", "quick_build",
]
