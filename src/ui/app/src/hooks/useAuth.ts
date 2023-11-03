import { useState } from "react"

export type OIDCProvider = {

}

export default function useAuth() {
    const [isAuthenticated, setIsAuthenticated] = useState(false)

    return {
        isAuthenticated,
        setIsAuthenticated
    }
}