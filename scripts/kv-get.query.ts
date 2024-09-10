const { key } = context.getRequest<{ key: string }>();

const value = await store.get(key);

context.respond({ value });
