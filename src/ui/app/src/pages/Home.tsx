import React from 'react'
import SearchOrCreate from '../components/SearchOrCreate'
import Grid from "@mui/joy/Grid/Grid"

const Home: React.FC = () => {
   return (
      <>
         <Grid container alignItems={"center"} justifyContent={"center"}>
            <SearchOrCreate />
         </Grid>
      </>
   )
}

export default Home
