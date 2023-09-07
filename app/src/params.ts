import queryString from "query-string";
import {writable} from "svelte/store";


const parseQueryString = () => queryString.parse(window.location.search);

export const params = writable(parseQueryString());

export default {};