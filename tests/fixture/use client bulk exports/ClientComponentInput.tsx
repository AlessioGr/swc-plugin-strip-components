'use client'

import './index.scss'
export { MyFn as MyFnOtherFile } from "../use client/ClientComponentInput"; // should be kept
import { MyFn as MyFnOtherFile2 } from "../use client/ClientComponentOutput"; // should be kept due to re-export below
export { MyFnOtherFile2 }  // should be kept
import { MyFn2 as MyFn2FromAnotherFile } from "../use client/ClientComponentInput"; // should be removed


const MyFn = () => {
    return <div>MyFn</div>
}

const MyFn2 = () => {
    return <div>MyFn2</div>
}

export const someVariable = 'someVariable'


export { MyFn, MyFn2 }
