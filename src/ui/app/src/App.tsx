import React from 'react'
import { Switch, Route } from 'react-router-dom'
import { Helmet } from 'react-helmet'

import './styles/index.scss'
import favicon from '../public/favicon.png'

import Home from './pages/Home'
import NotFound from './pages/404'
import Login from './pages/Login'
import { AppProps } from './@types'

import { OidcClient, OidcClientSettingsStore } from 'oidc-client-ts'
import { NamedOidcClient } from './auth'

const App: React.FC<Partial<AppProps>> = ({ login_path, oidc_providers, oauth_redirect_endpoint, ...props }) => {
   console.log('found props: ', props)

   const oidc_clients: NamedOidcClient[] = oidc_providers?.map(provider => {
      const oidcSettings = new OidcClientSettingsStore({ authority: provider.provider_url, client_id: provider.client_id, redirect_uri: oauth_redirect_endpoint ?? '', metadata: provider.provider_metadata })
      return [provider.provider_name, new OidcClient(oidcSettings)]
   }) || []

   return (
      <>
         <Helmet>
            <meta charSet='utf-8' />
            <title>🦞 rustlinks ⚙️</title>
            <link rel='icon' type='image/png' href={favicon} />
         </Helmet>
         <Switch>
            <Route path='/' component={Home} />
            {/* <Route path={login_path ?? '/login'} element={<Login oidc_clients={oidc_clients} />} /> */}
            <Route path='*' component={NotFound} />
         </Switch>
      </>
   )
}

export default App
