import { OIDCProvider } from "../auth"

export type AppProps = {
    location: any,
    context: any,
    oidc_providers: OIDCProvider[],
    oauth_redirect_endpoint: string,
    login_path: string
}