# swc-plugin-strip-components

This swc plugin can be used to do one (or both) of 2 things

## 1. Lobotomize React 'use client' components in your code

Sometimes, you may have to import files which import client components on the server. This happens a lot of payload, where the payload config contains both server-side code, as well as client component imports.

This swc plugin automatically deletes the contents of files marked with 'use client' and keeps ONLY functions / variables which are exported. Additionally, those are nullified.

That way, code which relies on those imports will not break, and those client components are effectively "disabled".

Example:

**Input**

```ts
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
```

**Output of this plugin:**
```ts
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

```

## 2. Nullify arguments to a specified function

This can be useful if this plugin is conditionally enabled, and you only want components (or any code, really) to be available on the client-only (so, if this swc plugin is not enabled).

If the identifier in your swc config is set to Component, this will be the example input / output:

**Input:**

```ts
type Component = <C>(
    C: C,
) => C | null

const Component: Component = (c) => c as any

{
    someProp: Component(MyComponent)
}
```

**Output:**

```ts
type Component = <C>(
    C: C,
) => C | null

const Component: Component = (c) => c as any

{
    someProp: Component(null)
}
```




## Installation

```bash
pnpm add swc-plugin-strip-components
```

then add it to your swc config:

```ts
const modifiedConfig = {
        ...swcRegisterConfig,
        swc: {
            ...{
                ...swcRegisterConfig?.swc ?? {}
            },
            jsc: {
                ...{
                    ...swcRegisterConfig?.swc?.jsc ?? {}
                },
                experimental: {
                    ...{
                        ...swcRegisterConfig?.swc?.jsc?.experimental ?? {}
                    },
                    plugins: [
                        ...swcRegisterConfig?.swc?.jsc?.experimental?.plugins ?? [],
                        [
                            'swc-plugin-strip-components',
                            {
                                identifier: 'Component', // the name of the function whose props should be nullified
                                lobotomize_use_client_files: true
                            }
                        ]
                    ]
                }
            }
        }
    };
```

Or in Next.js (if you want to break your entire app):

```ts
const nextConfig = {
  experimental: {
    swcPlugins: [
      [
        "swc-plugin-strip-components",
        {
          identifier: 'Component',
          lobotomize_use_client_files: false, // set to true, to break your entire next app
        },
      ],
    ]
  },
}
```
