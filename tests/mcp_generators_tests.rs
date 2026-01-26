use cakung_barat_server::mcp::generators::surat_kpr::{SuratKprGenerator, SuratKprRequest};
use cakung_barat_server::mcp::generators::surat_nib_npwp::{SuratNibNpwpGenerator, SuratNibNpwpRequest};
use cakung_barat_server::mcp::generators::surat_tidak_mampu::{SuratTidakMampuGenerator, SuratTidakMampuRequest};
use serde_json;

// SuratKpr Tests

#[test]
fn test_surat_kpr_new_generator() {
    let result = SuratKprGenerator::new();
    assert!(result.is_ok());
}

#[test]
fn test_surat_kpr_request_deserialization() {
    let json = r#"{
        "data": {
            "nama": "Jane Doe",
            "nik": "1234567890123456",
            "ttl": "Jakarta, 15 Maret 1985",
            "jk": false,
            "agama": "Kristen",
            "pekerjaan": "PNS",
            "alamat": "Jl. Melati No. 5",
            "telp": "08198765432"
        },
        "meta": {
            "kelurahan": "Cakung Barat",
            "bank_tujuan": "Bank BTN"
        }
    }"#;

    let request: SuratKprRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.data.nama, "Jane Doe");
    assert_eq!(request.meta.bank_tujuan, "Bank BTN");
}

// SuratNibNpwp Tests

#[test]
fn test_surat_nib_npwp_new_generator() {
    let result = SuratNibNpwpGenerator::new();
    assert!(result.is_ok());
}

#[test]
fn test_surat_nib_npwp_request_deserialization() {
    let json = r#"{
        "data": {
            "nama": "Ahmad Wirawan",
            "nik": "3171234567890123",
            "jabatan": "Pemilik",
            "bidang_usaha": "Perdagangan",
            "kegiatan_usaha": "Toko Kelontong",
            "jenis_usaha": "Usaha Mikro",
            "alamat_usaha": "Jl. Pasar No. 10"
        }
    }"#;

    let request: SuratNibNpwpRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.data.nama, "Ahmad Wirawan");
    assert_eq!(request.data.jenis_usaha, "Usaha Mikro");
}

// SuratTidakMampu Tests

#[test]
fn test_surat_tidak_mampu_new_generator() {
    // This test requires the template file to exist
    let result = SuratTidakMampuGenerator::new();
    assert!(result.is_ok());
}

#[test]
fn test_surat_tidak_mampu_request_deserialization() {
    let json = r#"{
        "pengisi": {
            "nama": "John Doe",
            "nik": "1234567890123456",
            "ttl": "Jakarta, 1 Januari 1990",
            "jk": true,
            "agama": "Islam",
            "pekerjaan": "Karyawan Swasta",
            "alamat": "Jl. Test No. 1",
            "telp": "08123456789"
        },
        "meta": {
            "opsi_sendiri": true,
            "kelurahan": "Cakung Barat"
        }
    }"#;

    let request: SuratTidakMampuRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.pengisi.nama, "John Doe");
    assert!(request.meta.opsi_sendiri);
}
