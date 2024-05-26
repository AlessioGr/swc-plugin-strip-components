type ClientOnlyFn = (<C>(
    C: C,
) => (C & { use_Component_helper_exported_from_payload_utilities: string }) | null)

export const ClientOnly: ClientOnlyFn = (c) => c as any
