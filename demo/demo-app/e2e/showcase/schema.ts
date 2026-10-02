/**
 * A TypeScript port of the JSON Schema subset in
 * `benchmarks/viabilitybench/schema/validate.py` (`check_schema`, `schema_errors`): type,
 * required, enum, const, properties, items, minItems and additionalProperties. A schema with
 * any other keyword is refused, so no rule is skipped in silence, and equality is JSON's
 * (false is not 0). The showcase schemas stay inside this subset so the Python bundle
 * builder can check bundles with `validate.py` against the same files.
 */

export type JsonSchema = Record<string, unknown>;

const KEYWORDS = new Set([
  'type',
  'required',
  'enum',
  'const',
  'properties',
  'items',
  'minItems',
  'additionalProperties',
]);
const ANNOTATIONS = new Set(['$schema', '$id', '$comment', 'title', 'description']);
const TYPES = ['null', 'boolean', 'integer', 'number', 'string', 'array', 'object'];

/** A schema uses something this validator does not implement. */
export class SchemaError extends Error {}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function typeList(schema: JsonSchema): string[] {
  const types = schema.type;
  if (types === undefined) return [];
  return typeof types === 'string' ? [types] : (types as string[]);
}

/** Throw `SchemaError` unless `schema` uses only the supported keywords, well formed. */
export function checkSchema(schema: unknown, path = '#'): void {
  if (!isObject(schema)) throw new SchemaError(`${path}: a schema must be an object`);
  const unsupported = Object.keys(schema).filter((k) => !KEYWORDS.has(k) && !ANNOTATIONS.has(k));
  if (unsupported.length > 0) {
    throw new SchemaError(`${path}: unsupported keyword(s): ${unsupported.sort().join(', ')}`);
  }
  const types = schema.type;
  if (types !== undefined && typeof types !== 'string' && !Array.isArray(types)) {
    throw new SchemaError(`${path}/type: must be a type name or a list of them`);
  }
  for (const name of typeList(schema)) {
    if (!TYPES.includes(name)) throw new SchemaError(`${path}/type: unknown type ${name}`);
  }
  const required = schema.required ?? [];
  if (!Array.isArray(required) || !required.every((name) => typeof name === 'string')) {
    throw new SchemaError(`${path}/required: must be a list of field names`);
  }
  if (!Array.isArray(schema.enum ?? [])) throw new SchemaError(`${path}/enum: must be a list`);
  const properties = schema.properties ?? {};
  if (!isObject(properties)) throw new SchemaError(`${path}/properties: must be an object`);
  const minItems = schema.minItems ?? 0;
  if (typeof minItems !== 'number' || !Number.isInteger(minItems) || minItems < 0) {
    throw new SchemaError(`${path}/minItems: must be a non-negative integer`);
  }
  for (const [name, sub] of Object.entries(properties)) {
    checkSchema(sub, `${path}/properties/${name}`);
  }
  if ('items' in schema) checkSchema(schema.items, `${path}/items`);
  const extra = schema.additionalProperties ?? true;
  if (isObject(extra)) {
    checkSchema(extra, `${path}/additionalProperties`);
  } else if (typeof extra !== 'boolean') {
    throw new SchemaError(`${path}/additionalProperties: must be a boolean or a schema`);
  }
}

function isType(value: unknown, name: string): boolean {
  switch (name) {
    case 'null':
      return value === null;
    case 'boolean':
      return typeof value === 'boolean';
    case 'integer':
      return typeof value === 'number' && Number.isInteger(value);
    case 'number':
      return typeof value === 'number' && Number.isFinite(value);
    case 'string':
      return typeof value === 'string';
    case 'array':
      return Array.isArray(value);
    case 'object':
      return isObject(value);
    default:
      return false;
  }
}

/** JSON equality: false is not 0, true is not 1, and 1 equals 1.0. */
export function sameJson(a: unknown, b: unknown): boolean {
  if (Array.isArray(a) || Array.isArray(b)) {
    if (!Array.isArray(a) || !Array.isArray(b) || a.length !== b.length) return false;
    return a.every((item, index) => sameJson(item, b[index]));
  }
  if (isObject(a) || isObject(b)) {
    if (!isObject(a) || !isObject(b)) return false;
    const keys = Object.keys(a);
    if (keys.length !== Object.keys(b).length) return false;
    return keys.every((key) => key in b && sameJson(a[key], b[key]));
  }
  return a === b;
}

function typeName(value: unknown): string {
  return TYPES.find((name) => isType(value, name)) ?? typeof value;
}

function show(value: unknown): string {
  return JSON.stringify(value) ?? String(value);
}

function child(path: string, key: string): string {
  const identifier = /^[A-Za-z_][A-Za-z0-9_]*$/.test(key);
  return identifier ? `${path}.${key}` : `${path}[${JSON.stringify(key)}]`;
}

/** Every way `value` breaks `schema`, as "<path>: <problem>" strings; empty when it conforms. */
export function schemaErrors(value: unknown, schema: JsonSchema, path = '$'): string[] {
  const types = typeList(schema);
  if (types.length > 0 && !types.some((name) => isType(value, name))) {
    return [`${path}: expected ${types.join(' or ')}, got ${typeName(value)}`];
  }
  const errors: string[] = [];
  if ('const' in schema && !sameJson(value, schema.const)) {
    errors.push(`${path}: must be ${show(schema.const)}, not ${show(value)}`);
  }
  const options = schema.enum as unknown[] | undefined;
  if (options !== undefined && !options.some((option) => sameJson(value, option))) {
    errors.push(`${path}: ${show(value)} is not one of ${show(options)}`);
  }
  if (isObject(value)) {
    for (const name of (schema.required as string[] | undefined) ?? []) {
      if (!(name in value)) errors.push(`${path}: missing required field '${name}'`);
    }
    const properties = (schema.properties as Record<string, JsonSchema> | undefined) ?? {};
    const extra = schema.additionalProperties ?? true;
    for (const [name, item] of Object.entries(value)) {
      if (name in properties) {
        errors.push(...schemaErrors(item, properties[name], child(path, name)));
      } else if (extra === false) {
        errors.push(`${child(path, name)}: unexpected field`);
      } else if (isObject(extra)) {
        errors.push(...schemaErrors(item, extra, child(path, name)));
      }
    }
  }
  if (Array.isArray(value)) {
    const minItems = (schema.minItems as number | undefined) ?? 0;
    if (value.length < minItems) {
      errors.push(`${path}: needs at least ${minItems} item(s), has ${value.length}`);
    }
    if ('items' in schema) {
      value.forEach((item, index) => {
        errors.push(...schemaErrors(item, schema.items as JsonSchema, `${path}[${index}]`));
      });
    }
  }
  return errors;
}

/** The distinct paths named by `errors` (the part before the first ": "). */
export function errorPaths(errors: string[]): string[] {
  return [...new Set(errors.map((error) => error.slice(0, error.indexOf(': '))))].sort();
}
