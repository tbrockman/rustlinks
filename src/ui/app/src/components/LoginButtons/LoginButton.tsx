import React from 'react'
import { OidcClient } from 'oidc-client-ts'

export type LoginButtonElement = JSX.Element

export type IconProps = {
    width: number | string,
    height: number | string,
}

export type ProviderLoginProps = Omit<LoginButtonProps, "icon"> & {
    icon?: JSX.Element
}

export type LoginButtonProps = React.PropsWithChildren<{
    oidc_client: OidcClient,
    [props: string]: any
}>


export default function LoginButton({ oidc_client, icon, children, props }: LoginButtonProps) {

    // const navigate = useNavigate()

    const navigate = (url: string) => {

    }

    const loginClicked = async () => {
        const request = await oidc_client.createSigninRequest({})
        console.log('created signin request', request)
        navigate(request.url)
    }

    console.log('login button props', props)
    return (
        <div key={props.key} onClick={loginClicked} {...props}>
            {icon}
            {children}
        </div>
    )
}