'use client';
export { MyFn as MyFnOtherFile } from "../use client/ClientComponentInput"; // should be kept
import { MyFn as MyFnOtherFile2 } from "../use client/ClientComponentOutput"; // should be kept due to re-export below
export { MyFnOtherFile2 } ; // should be kept
const MyFn = () =>null;
const MyFn2 = () =>null;
export const someVariable = null;
export { MyFn, MyFn2 };
