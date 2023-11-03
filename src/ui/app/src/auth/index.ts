import { OidcClient, OidcMetadata } from "oidc-client-ts"


export type NamedOidcClient = [string, OidcClient]

// TODO: either generate this type from Rust code or convert to a protobuf
export type OIDCProvider = {
    client_id: string,
    provider_url: string,
    provider_name: string,
    provider_metadata: OidcMetadata
}