import React from 'react'
import AppleLogin from '../components/LoginButtons/AppleLogin'
import GoogleLogin from '../components/LoginButtons/GoogleLogin'
import { LoginButtonElement, ProviderLoginProps } from '../components/LoginButtons/LoginButton'
import { OidcClient } from 'oidc-client-ts'


export type NamedOidcClient = [string, OidcClient]

type LoginProps = {
    oidc_clients: NamedOidcClient[],
    [key: string]: any
}

const buttonOIDCOptionMap = new Map<string, ({ ...args }: ProviderLoginProps) => LoginButtonElement>(
    [
        ["Google", GoogleLogin],
        ["Apple", AppleLogin]
    ])

export default function Login({ oidc_clients, ...props }: LoginProps) {

    console.log('in login coomponent??')

    const buttons = oidc_clients.map(([provider_name, oidc_client]) => {
        const constructor = buttonOIDCOptionMap.get(provider_name)
        return constructor ? constructor({ oidc_client }) : <></>
    })

    return (
        <>
            {buttons}
        </>
    )
}
