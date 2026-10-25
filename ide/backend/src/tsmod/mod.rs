//! tsmod — Total Security modules: injini HALISI za ulinzi, module moja kwa faili moja.
//!
//! Kila module ina algorithm inayofanya kazi (Rust) na, pale inapowezekana,
//! actuation halisi (nftables, sockets, syscalls). Hakuna catalog, stub,
//! simulator wala demo — kila kitu kinachorudishwa kinatokana na mfumo halisi.
//!
//! Kanuni za usalama (hazipitwi):
//!   - HITL: vitendo vinavyobadilisha mfumo vinahitaji `apply = true` (idhini).
//!   - Bila `apply`, module inarudisha `dry_run: true` + mabadiliko yaliyopangwa.
//!   - Hakuna shell ya hiari; amri ni arg-array pekee.
//!   - Ufuatiliaji wa siri (camera/mic/interception) HAUJENGWI — ni haramu.

pub mod engine;
pub mod common_calls;
pub mod m1_voice_call_encryption;
pub mod m2_call_spoofing_detection;
pub mod m3_call_blocking;
pub mod m4_call_forwarding_security;
pub mod m5_forwarding_fraud;
pub mod m6_call_drop_protection;
pub mod m7_call_quality;
pub mod m8_recording_protection;
pub mod m9_emergency_priority;
pub mod m10_international_fraud;
pub mod common_sms;
pub mod m11_sms_encryption;
pub mod m12_sms_phishing;
pub mod m13_sms_forwarding_security;
pub mod m14_sms_fraud;
pub mod m15_sms_dropped;
pub mod m16_premium_sms_blocking;
pub mod m17_sms_spoofing;
pub mod m18_sms_api_security;
pub mod m19_sms_auto_reply_fraud;
pub mod m20_sms_content_filtering;
pub mod common_ai;
pub mod m21_ml_engine;
pub mod m22_deep_learning;
pub mod m23_pattern_recognition;
pub mod m24_predictive_analytics;
pub mod m25_reinforcement_learning;
pub mod m26_realtime_monitoring;
pub mod m27_big_data;
pub mod m28_behavioral_analysis;
pub mod m29_network_traffic_analysis;
pub mod m30_endpoint_behavior;
pub mod m31_automated_response;
pub mod m32_threat_mitigation;
pub mod m33_self_healing;
pub mod m34_alerting;
pub mod common_ext;
pub mod common_twist;
pub mod twist_engine;
pub mod twists;
pub mod m501_data_recovery;
pub mod m502_dark_web_intelligence;
pub mod m503_iot_security;
pub mod m505_system_architecture;
pub mod m506_zero_trust;
pub mod m507_nine_layer_scan;
pub mod m508_africa_focus;
pub mod m509_future_proof;
pub mod m510_deception_honeypot;
pub mod m511_moving_target_defense;
pub mod m512_adversarial_ai;
pub mod m513_supply_chain_sbom;
pub mod m514_tee_attestation;
pub mod m515_qrng;
pub mod m516_continuous_learning;
pub mod m517_payment_callback_verify;
pub mod m35_firewall;
pub mod m36_ids;
pub mod m37_vpn;
pub mod m38_dns;
pub mod m39_dhcp;
pub mod m40_routing;
pub mod m41_switching;
pub mod m42_wifi;
pub mod m43_bluetooth;
pub mod m44_nfc;
pub mod m45_usb;
pub mod m46_printer;
pub mod m47_proxy;
pub mod m48_loadbalancer;
pub mod m49_cdn;
pub mod m50_apigateway;
pub mod m51_microservices;
pub mod m52_container;
pub mod m53_orchestration;
pub mod m54_cloud;
pub mod m55_edge;
pub mod m56_iot;
pub mod m57_ot;
pub mod m273_voice_encryption;
pub mod m274_video_encryption;
pub mod m275_spoofing_detection;
pub mod m276_caller_id_verification;
pub mod m277_robocall_protection;
pub mod m278_voicemail_security;
pub mod m279_recording_protection;
pub mod m280_conference_call_security;
pub mod m281_video_conference_security;
pub mod m282_webrtc_security;
pub mod m283_voip_security;
pub mod m284_mobile_network_call_security;
pub mod m285_landline_call_security;
pub mod m286_international_call_security;
pub mod m287_emergency_call_security;
pub mod common_smsemail;
pub mod m288_sms_encryption;
pub mod m289_sms_phishing;
pub mod m290_email_encryption;
pub mod m291_email_phishing;
pub mod m292_sms_spam;
pub mod m293_sms_sender_verification;
pub mod m294_sms_link_scanner;
pub mod m295_sms_attachment_security;
pub mod m296_email_spam;
pub mod m297_email_attachment_scanner;
pub mod m298_email_link_scanner;
pub mod m299_email_impersonation;
pub mod m300_email_bec_protection;
pub mod m301_email_dlp;
pub mod m302_email_archiving;
pub mod m303_email_compliance;
pub mod m304_email_audit_trail;
pub mod m305_email_key_management;
pub mod common_device;
pub mod m306_sim_swap_detection;
pub mod m307_device_fingerprinting;
pub mod m308_location_tracking;
pub mod m309_geolocation_security;
pub mod m310_remote_lock;
pub mod m311_remote_wipe;
pub mod m312_device_backup;
pub mod m313_device_restore;
pub mod m314_device_encryption;
pub mod m315_password_policy;
pub mod m316_screen_lock_security;
pub mod m317_app_permission_control;
pub mod m318_vpn_configuration;
pub mod m319_proxy_configuration;
pub mod m320_dns_security;
pub mod m321_firewall_settings;
pub mod m322_antivirus_integration;
pub mod common_human;
pub mod m323_peacekeeping_missions_protection;
pub mod m324_humanitarian_aid_workers;
pub mod m325_refugee_data_protection;
pub mod m326_global_crime_tracking;
pub mod m327_cyber_warfare_defense;
pub mod m328_future_proof_architecture;
pub mod common_blockchain;
pub mod m468_blockchain_core;
pub mod m469_consensus_module;
pub mod m471_evidence_blockchain;
pub mod m472_audit_blockchain;
pub mod m473_compliance_blockchain;
pub mod m476_threat_intelligence_blockchain;
pub mod m477_secure_data_sharing_blockchain;
pub mod m478_smart_contract_security_scanner;
pub mod m479_blockchain_validator_module;
pub mod m480_blockchain_node_management;
pub mod m481_blockchain_analytics;
pub mod m482_blockchain_explorer;
pub mod social;
pub mod store;

use serde_json::{json, Value};

/// Matokeo ya module moja. `findings` = yaliyogundulika; `actions` = yaliyofanyika
/// (au yangefanyika kwenye dry-run).
#[derive(Debug, Clone)]
pub struct ModReport {
    pub ok: bool,
    pub module: u16,
    pub name: &'static str,
    pub summary: String,
    pub findings: Vec<Value>,
    pub actions: Vec<Value>,
    pub applied: bool,
    pub dry_run: bool,
}

impl ModReport {
    pub fn new(module: u16, name: &'static str) -> Self {
        ModReport {
            ok: true,
            module,
            name,
            summary: String::new(),
            findings: Vec::new(),
            actions: Vec::new(),
            applied: false,
            dry_run: true,
        }
    }

    pub fn finding(&mut self, kind: &str, detail: impl Into<String>, severity: u8) -> &mut Self {
        self.findings.push(json!({
            "kind": kind, "detail": detail.into(), "severity": severity
        }));
        self
    }

    pub fn action(&mut self, kind: &str, detail: impl Into<String>, ok: bool) -> &mut Self {
        self.actions.push(json!({ "kind": kind, "detail": detail.into(), "ok": ok }));
        self
    }

    pub fn to_json(&self) -> Value {
        json!({
            "ok": self.ok,
            "module": self.module,
            "name": self.name,
            "summary": self.summary,
            "findings": self.findings,
            "actions": self.actions,
            "applied": self.applied,
            "dry_run": self.dry_run,
        })
    }
}

/// Module ya Total Security. `run` ni blocking (sockets/nft ni za haraka);
/// handler ya HTTP huiita kwenye `spawn_blocking`. Modules zinazohitaji DB
/// (k.m. kijamii) hutumia `run_db`.
#[async_trait::async_trait]
pub trait TsModule: Send + Sync {
    fn id(&self) -> u16;
    fn name(&self) -> &'static str;
    fn group(&self) -> &'static str;
    /// Maelezo mafupi ya kazi (Kiswahili) — huonyeshwa kwenye catalog.
    fn about(&self) -> &'static str;
    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String>;
    /// Toleo la async linalopata hifadhidata (default: huiita `run`).
    async fn run_db(
        &self,
        _db: &sqlx::SqlitePool,
        target: &str,
        params: &Value,
    ) -> Result<ModReport, String> {
        self.run(target, params)
    }
}

/// Rejista ya modules zote zilizojengwa. Huongezwa module kwa module.
pub fn registry() -> &'static [&'static dyn TsModule] {
    &[
        &m1_voice_call_encryption::M1VoiceCallEncryption,
        &m2_call_spoofing_detection::M2CallSpoofingDetection,
        &m3_call_blocking::M3CallBlocking,
        &m4_call_forwarding_security::M4CallForwardingSecurity,
        &m5_forwarding_fraud::M5ForwardingFraud,
        &m6_call_drop_protection::M6CallDropProtection,
        &m7_call_quality::M7CallQuality,
        &m8_recording_protection::M8RecordingProtection,
        &m9_emergency_priority::M9EmergencyPriority,
        &m10_international_fraud::M10InternationalFraud,
        &m11_sms_encryption::M11SmsEncryption,
        &m12_sms_phishing::M12SmsPhishing,
        &m13_sms_forwarding_security::M13SmsForwardingSecurity,
        &m14_sms_fraud::M14SmsFraud,
        &m15_sms_dropped::M15SmsDropped,
        &m16_premium_sms_blocking::M16PremiumSmsBlocking,
        &m17_sms_spoofing::M17SmsSpoofing,
        &m18_sms_api_security::M18SmsApiSecurity,
        &m19_sms_auto_reply_fraud::M19SmsAutoReplyFraud,
        &m20_sms_content_filtering::M20SmsContentFiltering,
        &m21_ml_engine::M21MlEngine,
        &m22_deep_learning::M22DeepLearning,
        &m23_pattern_recognition::M23PatternRecognition,
        &m24_predictive_analytics::M24PredictiveAnalytics,
        &m25_reinforcement_learning::M25ReinforcementLearning,
        &m26_realtime_monitoring::M26RealtimeMonitoring,
        &m27_big_data::M27BigData,
        &m28_behavioral_analysis::M28BehavioralAnalysis,
        &m29_network_traffic_analysis::M29NetworkTraffic,
        &m30_endpoint_behavior::M30EndpointBehavior,
        &m31_automated_response::M31AutomatedResponse,
        &m32_threat_mitigation::M32ThreatMitigation,
        &m33_self_healing::M33SelfHealing,
        &m34_alerting::M34Alerting,
        &m501_data_recovery::M501DataRecovery,
        &m502_dark_web_intelligence::M502DarkWeb,
        &m503_iot_security::M503IotSecurity,
        &m505_system_architecture::M505SystemArchitecture,
        &m506_zero_trust::M506ZeroTrust,
        &m507_nine_layer_scan::M507NineLayer,
        &m508_africa_focus::M508AfricaFocus,
        &m509_future_proof::M509FutureProof,
        &m510_deception_honeypot::M510DeceptionHoneypot,
        &m511_moving_target_defense::M511MovingTarget,
        &m512_adversarial_ai::M512AdversarialAi,
        &m513_supply_chain_sbom::M513SupplyChainSbom,
        &m514_tee_attestation::M514TeeAttestation,
        &m515_qrng::M515Qrng,
        &m516_continuous_learning::M516ContinuousLearning,
        &m35_firewall::M35Firewall,
        &m36_ids::M36Ids,
        &m37_vpn::M37Vpn,
        &m38_dns::M38Dns,
        &m39_dhcp::M39Dhcp,
        &m40_routing::M40Routing,
        &m41_switching::M41Switching,
        &m42_wifi::M42Wifi,
        &m43_bluetooth::M43Bluetooth,
        &m44_nfc::M44Nfc,
        &m45_usb::M45Usb,
        &m46_printer::M46Printer,
        &m47_proxy::M47Proxy,
        &m48_loadbalancer::M48LoadBalancer,
        &m49_cdn::M49Cdn,
        &m50_apigateway::M50ApiGateway,
        &m51_microservices::M51Microservices,
        &m52_container::M52Container,
        &m53_orchestration::M53Orchestration,
        &m54_cloud::M54Cloud,
        &m55_edge::M55Edge,
        &m56_iot::M56Iot,
        &m57_ot::M57Ot,
        &m273_voice_encryption::M273VoiceEncryption,
        &m274_video_encryption::M274VideoEncryption,
        &m275_spoofing_detection::M275SpoofingDetection,
        &m276_caller_id_verification::M276CallerIdVerification,
        &m277_robocall_protection::M277RobocallProtection,
        &m278_voicemail_security::M278VoicemailSecurity,
        &m279_recording_protection::M279RecordingProtection,
        &m280_conference_call_security::M280ConferenceCallSecurity,
        &m281_video_conference_security::M281VideoConferenceSecurity,
        &m282_webrtc_security::M282WebRtcSecurity,
        &m283_voip_security::M283VoipSecurity,
        &m284_mobile_network_call_security::M284MobileNetworkCallSecurity,
        &m285_landline_call_security::M285LandlineCallSecurity,
        &m286_international_call_security::M286InternationalCallSecurity,
        &m287_emergency_call_security::M287EmergencyCallSecurity,
        &m288_sms_encryption::M288SmsEncryption,
        &m289_sms_phishing::M289SmsPhishing,
        &m290_email_encryption::M290EmailEncryption,
        &m291_email_phishing::M291EmailPhishing,
        &m292_sms_spam::M292SmsSpam,
        &m293_sms_sender_verification::M293SmsSenderVerification,
        &m294_sms_link_scanner::M294SmsLinkScanner,
        &m295_sms_attachment_security::M295SmsAttachmentSecurity,
        &m296_email_spam::M296EmailSpam,
        &m297_email_attachment_scanner::M297EmailAttachmentScanner,
        &m298_email_link_scanner::M298EmailLinkScanner,
        &m299_email_impersonation::M299EmailImpersonation,
        &m300_email_bec_protection::M300EmailBecProtection,
        &m301_email_dlp::M301EmailDlp,
        &m302_email_archiving::M302EmailArchiving,
        &m303_email_compliance::M303EmailCompliance,
        &m304_email_audit_trail::M304EmailAuditTrail,
        &m305_email_key_management::M305EmailKeyManagement,
        &m306_sim_swap_detection::M306SimSwapDetection,
        &m307_device_fingerprinting::M307DeviceFingerprinting,
        &m308_location_tracking::M308LocationTracking,
        &m309_geolocation_security::M309GeolocationSecurity,
        &m310_remote_lock::M310RemoteLock,
        &m311_remote_wipe::M311RemoteWipe,
        &m312_device_backup::M312DeviceBackup,
        &m313_device_restore::M313DeviceRestore,
        &m314_device_encryption::M314DeviceEncryption,
        &m315_password_policy::M315PasswordPolicy,
        &m316_screen_lock_security::M316ScreenLockSecurity,
        &m317_app_permission_control::M317AppPermissionControl,
        &m318_vpn_configuration::M318VpnConfiguration,
        &m319_proxy_configuration::M319ProxyConfiguration,
        &m320_dns_security::M320DnsSecurity,
        &m321_firewall_settings::M321FirewallSettings,
        &m322_antivirus_integration::M322AntivirusIntegration,
        &m323_peacekeeping_missions_protection::M323PeacekeepingMissionsProtection,
        &m324_humanitarian_aid_workers::M324HumanitarianAidWorkers,
        &m325_refugee_data_protection::M325RefugeeDataProtection,
        &m326_global_crime_tracking::M326GlobalCrimeTracking,
        &m327_cyber_warfare_defense::M327CyberWarfareDefense,
        &m328_future_proof_architecture::M328FutureProofArchitecture,
        &m468_blockchain_core::M468BlockchainCore,
        &m469_consensus_module::M469ConsensusModule,
        &m471_evidence_blockchain::M471EvidenceBlockchain,
        &m472_audit_blockchain::M472AuditBlockchain,
        &m473_compliance_blockchain::M473ComplianceBlockchain,
        &m476_threat_intelligence_blockchain::M476ThreatIntelligenceBlockchain,
        &m477_secure_data_sharing_blockchain::M477SecureDataSharingBlockchain,
        &m478_smart_contract_security_scanner::M478SmartContractSecurityScanner,
        &m479_blockchain_validator_module::M479BlockchainValidatorModule,
        &m480_blockchain_node_management::M480BlockchainNodeManagement,
        &m481_blockchain_analytics::M481BlockchainAnalytics,
        &m482_blockchain_explorer::M482BlockchainExplorer,
        &m517_payment_callback_verify::M517PaymentCallbackVerify,
    ]
}

/// Tafuta module yoyote kwa id (core 35-57 au kijamii 58-272).
pub fn find(id: u16) -> Option<&'static dyn TsModule> {
    if let Some(m) = registry().iter().copied().find(|m| m.id() == id) {
        return Some(m);
    }
    if let Some(m) = twists::all().iter().copied().find(|m| m.id() == id) {
        return Some(m);
    }
    // Module za kijamii ni Copy + 'static — zinaishi kwenye rejista ya kudumu.
    social::all()
        .iter()
        .find(|m| m.id == id)
        .map(|m| m as &'static dyn TsModule)
}

/// Catalog kamili: modules 35-57 (core) + 58-272 (kijamii) + twists 1031-1300.
pub fn catalog_full() -> Vec<Value> {
    let mut v: Vec<Value> = registry()
        .iter()
        .map(|m| {
            json!({
                "module": m.id(), "name": m.name(), "group": m.group(), "about": m.about(),
            })
        })
        .collect();
    v.extend(twists::all().iter().map(|m| {
        json!({
            "module": m.id(), "name": m.name(), "group": m.group(), "about": m.about(),
        })
    }));
    v.extend(social::catalog());
    v.sort_by_key(|x| x["module"].as_u64().unwrap_or(0));
    v
}

pub fn catalog_json() -> Value {
    let items = catalog_full();
    json!({ "ok": true, "count": items.len(), "modules": items })
}
