const { key, value } = context.getRequest<{ key: string; value: string }>();

await store.set(key, value);

context.emit({
  type: "kv-set",
  attributes: [{ key, value, index: false }],
});
