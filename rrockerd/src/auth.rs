use tonic::{Request, Status};
use x509_parser::prelude::X509Certificate;

#[derive(Debug)]
/// The request's authorization
pub struct ClientAuth {
    //in a production system you'd convert both the id and group to integer based ids asap
    //for perf reasons but in simplicity's name I'm cutting that corner
    pub id: String,
    pub group: String,
}

/// Interceptor used to check the certificate of a request has a valid organization name
#[tracing::instrument]
pub(crate) fn authorization_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    let peer_certs = req
        .peer_certs()
        .ok_or(Status::unauthenticated("Missing certs"))?;

    let (_, cert) = peer_certs
        .iter()
        .map(|c| x509_parser::parse_x509_certificate(c.get_ref()))
        .next()
        .ok_or(Status::unauthenticated("Empty cert list"))?
        .map_err(|_| Status::unauthenticated("One or more certs are invalid"))?;

    validate_cert(cert, req)
}

#[tracing::instrument]
/// Split out from authorization_interceptor to make it testable
fn validate_cert(cert: X509Certificate, mut req: Request<()>) -> Result<Request<()>, Status> {
    let common_name = cert
        .subject()
        .iter_common_name()
        .next()
        .ok_or(Status::unauthenticated("Cert doesn't contain common name"))?;
    let common_name = common_name
        .as_str()
        .map_err(|_| Status::unauthenticated("Invalid common name"))?;
    let org_name = cert
        .subject()
        .iter_organization()
        .next()
        .ok_or(Status::unauthenticated("Cert doesn't contain organization"))?;
    let org_name = org_name
        .as_str()
        .map_err(|_| Status::unauthenticated("Invalid organization"))?;

    const VALID_ORG_NAMES: &[&str] = &["client", "admin"];

    if VALID_ORG_NAMES.contains(&org_name) {
        req.extensions_mut().insert(ClientAuth {
            id: common_name.to_owned(),
            group: org_name.to_owned(),
        });

        Ok(req)
    } else {
        tracing::warn!("Received request with invalid org/group: {}", org_name);
        Err(Status::unauthenticated(
            "The organization of the provided cert is not valid",
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tonic::Code;
    use x509_parser::pem::Pem;

    #[test]
    fn test_missing_cert() {
        //this would've been nicer if Request and Status implemented Eq :(
        let status = authorization_interceptor(Request::new(())).unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
    }

    #[test]
    fn test_good_name() {
        const GOOD_CERT: &[u8] = include_bytes!("../../certs/test/good_crt.pem");
        let pem = Pem::iter_from_buffer(GOOD_CERT).next().unwrap().unwrap();
        let invalid_cert = pem.parse_x509().unwrap();
        let _ = validate_cert(invalid_cert, Request::new(())).unwrap();
    }
    #[test]
    fn test_invalid_certs() {
        const MISSING_CN_CERT: &[u8] = include_bytes!("../../certs/test/missing_cn_crt.pem");
        const INVALID_ORG_NAME_CERT: &[u8] =
            include_bytes!("../../certs/test/invalid_org_name_crt.pem");
        const MISSING_ORG_NAME_CERT: &[u8] =
            include_bytes!("../../certs/test/missing_org_name_crt.pem");

        for cert in [
            MISSING_CN_CERT,
            INVALID_ORG_NAME_CERT,
            MISSING_ORG_NAME_CERT,
        ] {
            let pem = Pem::iter_from_buffer(cert).next().unwrap().unwrap();
            let invalid_cert = pem.parse_x509().unwrap();
            let status = validate_cert(invalid_cert, Request::new(())).unwrap_err();
            assert_eq!(status.code(), Code::Unauthenticated);
        }
    }
}
