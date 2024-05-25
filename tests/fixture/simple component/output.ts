type ComponentFn = (C: any) => any

const Component: ComponentFn = (C) => {
    return C
}

const test = {
    hello: Component(null)
};
