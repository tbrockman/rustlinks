import React from 'react'
import { Switch, Route } from 'react-router-dom'
import { Helmet } from 'react-helmet'

import './styles/index.scss'
import favicon from '../public/favicon.png'

import Home from './pages/Home'
import NotFound from './pages/404'

const App: React.FC = () => {
   return (
      <>
         <Helmet>
            <meta charSet='utf-8' />
            <title>🦞 rustlinks ⚙️</title>
            <link rel='icon' type='image/png' href={favicon} />
         </Helmet>
         <Switch>
            <Route exact path='/' component={Home} />
            <Route path='*' component={NotFound} />
         </Switch>
      </>
   )
}

export default App
