'use client';
export { MyFn as MyFnOtherFile } from "../use client/ClientComponentInput"; // should be kept, due to it being exported
import { MyFn as MyFnOtherFile2 } from "../use client/ClientComponentOutput"; // should be kept due to re-export below
export { MyFnOtherFile2 }; // should be kept, due to it being exported
const MyFn = ()=>null// Should be kept due to exported below
;
const MyFn2 = ()=>null// Should be kept due to exported below
;
export function MyFn4() {
    return null;
} // Should be kept due to exported directly
function MyFn5() {
    return null;
} // Should be kept due to exported below
export const someVariable = null// Should be kept. Exported directly
;
const someVariable2 = null// Should be kept. Exported below
;
export { MyFn, MyFn2, MyFn5, someVariable2 };
