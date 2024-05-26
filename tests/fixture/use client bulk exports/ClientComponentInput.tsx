'use client'

import './index.scss' // Should be removed
export { MyFn as MyFnOtherFile } from "../use client/ClientComponentInput"; // should be kept, due to it being exported
import { MyFn as MyFnOtherFile2 } from "../use client/ClientComponentOutput"; // should be kept due to re-export below
export { MyFnOtherFile2 }  // should be kept, due to it being exported
import { MyFn2 as MyFn2FromAnotherFile } from "../use client/ClientComponentInput"; // should be removed. Not exported and not used anywhere


import { lazy } from 'react' // Should be removed


const RichTextEditor = lazy(() =>
    // @ts-ignore
    import('../use client/ClientComponentOutput').then((module) => ({ default: module.MyFn })),
) // Should be removed completely.


const MyFn = () => {
    return <div>MyFn</div>
} // Should be kept due to exported below

const MyFn2 = () => {
    return <div>MyFn2</div>
} // Should be kept due to exported below

export function MyFn4 () {
    return <div>MyFn3</div>
} // Should be kept due to exported directly


function MyFn5 () {
    return <div>MyFn3</div>
} // Should be kept due to exported below

function MyFn6 () { // Removed
    return <div>MyFn3</div>
} // Should be removed completely. Not exported and not exported below

export const someVariable = 'someVariable' // Should be kept. Exported directly

const someVariable2 = 'someVariable2' // Should be kept. Exported below

const someVariable3 = 'someVariable3' // Should be removed


export { MyFn, MyFn2, MyFn5, someVariable2 }
