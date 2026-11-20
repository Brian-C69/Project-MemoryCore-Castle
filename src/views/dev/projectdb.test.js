import { describe, expect, it } from 'vitest'
import { chipOf, filterTables, formOf, formToSave, fromWords, kindOf, quote, rowsLabel, savedWith, savedWithout, secondsLeft, shortHost, tableLook, updateFor } from './projectdb.js'

describe('the Database panel', () => {
  it('reads the kind of the statement past comments', () => {
    expect(kindOf('SELECT 1')).toBe('read')
    expect(kindOf('-- note\n show tables')).toBe('read')
    expect(kindOf('/* x */ WITH t AS (SELECT 1) SELECT * FROM t')).toBe('read')
    expect(kindOf('WITH t AS (SELECT 1) DELETE FROM x')).toBe('write')
    expect(kindOf('update trips set a = 1')).toBe('write')
    expect(kindOf('DROP TABLE trips')).toBe('shape')
    expect(kindOf('   ')).toBe('')
  })

  it('makes the UPDATE of one edited cell, only with a primary key', () => {
    const columns = [{ name: 'id', key: true }, { name: 'status', key: false }]
    const grid = ['id', 'status']
    expect(updateFor('trips', columns, grid, ['7', 'open'], 1, "it's done")).toBe("UPDATE `trips` SET `status` = 'it\\'s done' WHERE `id` = '7' LIMIT 1;")
    expect(updateFor('trips', columns, grid, ['7', 'open'], 1, null)).toBe("UPDATE `trips` SET `status` = NULL WHERE `id` = '7' LIMIT 1;")
    expect(updateFor('trips', [{ name: 'status', key: false }], grid, ['7', 'open'], 1, 'x')).toBe(null)
    expect(updateFor('trips', columns, ['status'], ['open'], 0, 'x')).toBe(null)
    expect(quote('a\nb')).toBe("'a\\nb'")
    expect(tableLook('we`ird')).toBe('SELECT * FROM `we``ird` LIMIT 100;')
  })

  it('says the rows and the time, and keeps saved queries by name', () => {
    expect(chipOf({ columns: ['a'], rows: [[1], [2]], ms: 80 })).toBe('2 rows · 0.08 s')
    expect(chipOf({ columns: ['a'], rows: new Array(500).fill([1]), truncated: true, ms: 1200 })).toBe('first 500 rows · 1.20 s')
    expect(chipOf({ columns: [], rows: [], affected: 3 })).toBe('3 rows changed')
    const list = savedWith([], "Today's trips", 'SELECT 1')
    expect(savedWith(list, "today's trips", 'SELECT 2')).toEqual([{ name: "today's trips", sql: 'SELECT 2' }])
    expect(savedWith(list, ' ', 'x')).toBe(list)
    expect(savedWithout(list, "Today's trips")).toEqual([])
    expect(secondsLeft(0, 30_000)).toBe(90)
    expect(secondsLeft(0, 500_000)).toBe(0)
  })
})

describe('the table list and the connection bar', () => {
  it('narrows the tables by any part of the name, in order', () => {
    const tables = ['activity_logs', 'invoices', 'Invoice_items', 'users']
    expect(filterTables(tables, 'inv')).toEqual(['invoices', 'Invoice_items'])
    expect(filterTables(tables, '  LOGS ')).toEqual(['activity_logs'])
    expect(filterTables(tables, '')).toEqual(tables)
    expect(filterTables(null, 'x')).toEqual([])
  })

  it('says a row estimate briefly', () => {
    expect(rowsLabel(86)).toBe('86')
    expect(rowsLabel(1248)).toBe('1,248')
    expect(rowsLabel(12_400)).toBe('12.4k')
    expect(rowsLabel(20_000)).toBe('20k')
    expect(rowsLabel(3_100_000)).toBe('3.1M')
    expect(rowsLabel(0)).toBe('0')
    expect(rowsLabel(undefined)).toBe('')
  })

  it('shortens a long cloud host and leaves an address alone', () => {
    expect(shortHost('my-cluster-db-mysql-sgp1-12345-do-user-1-0.j.db.ondigitalocean.com')).toBe('db.ondigitalocean.com')
    expect(shortHost('127.0.0.1')).toBe('127.0.0.1')
    expect(shortHost('localhost')).toBe('localhost')
    expect(shortHost(null)).toBe('')
  })
})

describe('the connection form', () => {
  it('starts from what Castle knows, the password always empty', () => {
    const local = formOf({ host: '127.0.0.1', port: 3307, database: 'shop', user: 'root', from: '.env', hasPassword: true }, 'local')
    expect(local).toEqual({ host: '127.0.0.1', port: 3307, database: 'shop', user: 'root', password: '', caPath: '' })
    expect(formOf({ from: 'none' }, 'production')).toMatchObject({ host: '', port: 25060, user: '' })
    expect(formOf(null, 'local')).toMatchObject({ host: '127.0.0.1', port: 3306, user: 'root' })
  })

  it('sends only what was typed, and refuses what cannot work', () => {
    const base = { host: ' db.example ', port: '25060', database: ' app ', user: 'doadmin', password: '', caPath: '' }
    expect(formToSave(base, 'production', true)).toEqual({ form: { host: 'db.example', port: 25060, database: 'app', user: 'doadmin' } })
    expect(formToSave(base, 'production', false).problem).toMatch(/password/)
    expect(formToSave({ ...base, password: 's3cret', caPath: 'C:/ca.crt' }, 'production', false).form).toMatchObject({ password: 's3cret', caPath: 'C:/ca.crt' })
    expect(formToSave({ ...base, caPath: 'C:/ca.crt' }, 'local', false).form.caPath).toBeUndefined()
    expect(formToSave({ ...base, host: 'db example' }, 'local', true).problem).toMatch(/host/)
    expect(formToSave({ ...base, user: ' ' }, 'local', true).problem).toMatch(/user/)
    expect(formToSave({ ...base, port: 'abc' }, 'local', true).problem).toMatch(/port/)
  })

  it('says where a login came from', () => {
    expect(fromWords('.env')).toBe("the project's .env")
    expect(fromWords('saved')).toBe('saved in Castle')
    expect(fromWords('none')).toBe('not set yet')
  })
})
