import Autocomplete from "@mui/joy/Autocomplete"
import AutocompleteOption from '@mui/joy/AutocompleteOption'
import FormLabel from "@mui/joy/FormLabel"
import FormControl from "@mui/joy/FormControl"
import React, { useEffect, useRef, useState } from "react"
import Grid from "@mui/joy/Grid/Grid"
import CircularProgress from "@mui/joy/CircularProgress"
import { createFilterOptions } from "@mui/joy/Autocomplete"

type Rustlink = {
    alias: string,
    url: string,
}


export default function SearchOrCreate() {
    const ref = useRef<HTMLInputElement>(null)
    const [loading, setLoading] = React.useState<boolean>(false)
    const [inputValue, setInputValue] = React.useState<string>('')
    const [link, setLink] = React.useState<Rustlink | null>(null)
    const [options, setOptions] = useState<Rustlink[]>([])

    useEffect(() => {
        if (inputValue == '') {
            ref.current?.focus()
        }
        // TODO: debounce
        if (inputValue.length > 1) {
            // fetch(`/api/links/search`, { method: 'POST', body: JSON.stringify({ query: inputValue }) })
            //     .then(response => response.json())
            //     .then(({ results }: { results: ShortLinkDataWithoutViews[] }) => {
            //         const mapped = results.map(link => {
            //             return {
            //                 short: new URLWithoutProtocol(link.short),
            //                 long: new URLWithoutProtocol(link.long)
            //             }
            //         })
            //         setOptions(mapped)
            //     })
            //     .catch(error => console.error(error))
        }
    }, [inputValue])

    const getLink = (url: string) => {
        setLoading(true)
        // fetch(`/api/links/${url}`)
        //     .then(response => response.json())
        //     .then((data: ShortLinkData) => {
        //         setLink({
        //             short: new URLWithoutProtocol(data.short),
        //             long: new URLWithoutProtocol(data.long),
        //             views: data.views
        //         })
        //         setLoading(false)
        //     })
    }

    const createLink = (alias: string, url: string) => {
        setLoading(true)
        // fetch('/api/links', {
        //     method: 'POST',
        //     body: JSON.stringify({ url })
        // })
        //     .then(response => response.json())
        //     .then((data: ShortLink) => {
        //         data.short = new URLWithoutProtocol(data.short)
        //         data.long = new URLWithoutProtocol(data.long)
        //         setLink(data)
        //         setLoading(false)
        //     })
        //     .catch(error => {
        //         console.error(error)
        //         setLoading(false)
        //         setLink(null)
        //     })
    }

    const filterOptions = createFilterOptions<[]>({
        stringify: (option) => URL.prototype.toString.apply(option),
    })

    return (
        <Grid container marginTop={'30vh'} spacing={1} flexDirection={'column'} maxWidth={'600px'}>
            <Grid maxWidth='100%'>
                <FormControl id="find-or-shorten-form">
                    <FormLabel style={{ fontSize: '28px', marginBottom: '7px' }}>🔍 Find or create 🦞 rustlinks 🔗</FormLabel>
                    <Autocomplete
                        placeholder={'your.link/here'}
                        value={link?.url || null}
                        inputValue={inputValue}
                        loading={loading}
                        onInputChange={(_, newInputValue) => {
                            setInputValue(newInputValue)
                        }}
                        onChange={(event, newValue, reason) => {
                            console.log(event, newValue, reason)
                            if (reason === 'selectOption' && newValue) {
                                // It will be an object if it's an option which was returned by search
                                if (typeof newValue === 'object') {

                                    if (newValue.url) {
                                        getLink(newValue.url)
                                    }
                                }
                            }
                            else if (reason === 'clear') {
                                setLink(null)
                                setOptions([])
                            }
                            else if (reason === 'createOption' && typeof newValue === 'string') {
                                // createLink(newValue)
                            }
                        }}
                        style={{ padding: '0 24px' }}
                        isOptionEqualToValue={(option, value) => option?.alias === value?.alias}
                        options={options}
                        freeSolo
                        selectOnFocus
                        handleHomeEndKeys
                        autoFocus
                        filterSelectedOptions
                        autoHighlight
                        autoSelect
                        endDecorator={
                            loading ? (
                                <CircularProgress size="sm" sx={{ bgcolor: 'background.surface' }} />
                            ) : null
                        }
                        clearOnEscape
                        slotProps={{
                            input: {
                                ref: ref
                            }
                        }}
                        renderOption={(props, option) => <AutocompleteOption {...props} />}
                        sx={{ width: 600, maxWidth: '100%', borderRadius: '12px' }}
                        getOptionLabel={(option) =>
                            typeof option === 'string' ? option : ''
                        }
                        filterOptions={(options, params) => {

                            // options = filterOptions(options, params)

                            // if (params.inputValue !== '' && params.inputValue.length > 2) {
                            //     options.push({
                            //         short: null,
                            //         long: params.inputValue,
                            //         text: `Shorten "${params.inputValue}"`,
                            //         views: {
                            //             today: 0,
                            //             week: 0,
                            //             all: 0
                            //         }
                            //     })
                            // }
                            return options
                        }}
                    />
                </FormControl>
            </Grid>
            <Grid>
            </Grid>
        </Grid>
    )
}