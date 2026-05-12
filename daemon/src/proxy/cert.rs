use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rcgen::{CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

static CERT_CACHE: Lazy<Mutex<HashMap<String, Arc<CertifiedKey>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

const CA_CERT_REL: &str = "certs/ca.crt";
const CA_KEY_REL: &str = "certs/ca.key";

pub fn ensure_root_ca(data_dir: &Path) -> (rcgen::Certificate, rcgen::KeyPair) {
    let cert_path = data_dir.join(CA_CERT_REL);
    let key_path = data_dir.join(CA_KEY_REL);

    if cert_path.exists() && key_path.exists() {
        tracing::info!(path = %cert_path.display(), "cert: loaded existing root CA");
        let ca_key_pem = fs::read_to_string(&key_path).unwrap();
        let ca_key = KeyPair::from_pem(&ca_key_pem).unwrap();

        let mut params = CertificateParams::new(vec![]).unwrap();
        params.distinguished_name.push(DnType::CommonName, "GisoNet CA");
        params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

        let ca_cert = params.self_signed(&ca_key).unwrap();
        return (ca_cert, ca_key);
    }

    tracing::info!(path = %cert_path.display(), "cert: generating new root CA");
    fs::create_dir_all(data_dir.join("certs")).unwrap();

    let mut params = CertificateParams::new(vec![]).unwrap();
    params.distinguished_name.push(DnType::CommonName, "GisoNet CA");
    params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    params.key_usages.push(KeyUsagePurpose::DigitalSignature);
    params.key_usages.push(KeyUsagePurpose::KeyCertSign);
    params.key_usages.push(KeyUsagePurpose::CrlSign);

    let ca_key = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
    let ca_cert = params.self_signed(&ca_key).unwrap();

    fs::write(&cert_path, ca_cert.pem()).unwrap();
    fs::write(&key_path, ca_key.serialize_pem()).unwrap();

    (ca_cert, ca_key)
}

pub struct DynamicCertResolver {
    pub ca_key: rcgen::KeyPair,
    pub data_dir: PathBuf,
}

impl fmt::Debug for DynamicCertResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicCertResolver")
            .field("ca_key", &"<rcgen::KeyPair>")
            .field("data_dir", &self.data_dir)
            .finish()
    }
}

impl ResolvesServerCert for DynamicCertResolver {
    fn resolve(&self, hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let domain = hello.server_name()?.to_string();

        if let Some(cached) = CERT_CACHE.lock().get(&domain) {
            tracing::trace!(domain, "cert: cache hit");
            return Some(cached.clone());
        }

        let cert_path = self.data_dir.join("certs").join(&domain).join("cert.pem");
        let key_path = self.data_dir.join("certs").join(&domain).join("key.pem");

        if cert_path.exists() && key_path.exists() {
            tracing::info!(domain, path = %cert_path.display(), "cert: loaded from disk");
            let cert = vec![CertificateDer::from_pem_file(&cert_path).ok()?];
            let key = PrivateKeyDer::from_pem_file(&key_path).ok()?;
            let ck = Arc::new(CertifiedKey::new(cert, rustls::crypto::ring::sign::any_supported_type(&key).ok()?));
            CERT_CACHE.lock().insert(domain, ck.clone());
            return Some(ck);
        }

        tracing::info!(domain, "cert: generating new leaf certificate");
        let key = KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256).unwrap();
        let mut params = CertificateParams::new(vec![domain.clone()]).unwrap();
        params.distinguished_name.push(DnType::CommonName, &domain);
        params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];

        let mut issuer_params = CertificateParams::new(vec![]).unwrap();
        issuer_params.distinguished_name.push(DnType::CommonName, "Dynamic CA");
        let issuer = Issuer::from_params(&issuer_params, &self.ca_key);
        let cert = params.signed_by(&key, &issuer).unwrap();

        fs::create_dir_all(cert_path.parent().unwrap()).ok();
        fs::write(&cert_path, cert.pem()).ok();
        fs::write(&key_path, key.serialize_pem()).ok();

        let rustls_cert = vec![CertificateDer::from_pem_file(&cert_path).unwrap()];
        let rustls_key = PrivateKeyDer::from_pem_file(&key_path).unwrap();

        let ck = Arc::new(CertifiedKey::new(
            rustls_cert,
            rustls::crypto::ring::sign::any_supported_type(&rustls_key).unwrap(),
        ));
        CERT_CACHE.lock().insert(domain, ck.clone());
        Some(ck)
    }
}
