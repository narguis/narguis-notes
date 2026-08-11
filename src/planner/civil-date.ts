export type CivilDate = { value: string }

const DAYS_IN_MONTH = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]

function daysInMonth(year: number, month: number): number | null {
  const days = DAYS_IN_MONTH[month - 1]
  if (days === undefined || month === 0 || month > 12) return null
  return month === 2 && year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0) ? 29 : days
}

export function parseCivilDate(value: unknown): CivilDate | null {
  if (typeof value !== "string" || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return null
  const year = Number(value.slice(0, 4))
  const month = Number(value.slice(5, 7))
  const day = Number(value.slice(8, 10))
  const maximum = daysInMonth(year, month)
  return maximum !== null && day >= 1 && day <= maximum ? { value } : null
}

export function shiftCivilDate(date: CivilDate, offset: -1 | 1): CivilDate {
  const [yearText, monthText, dayText] = date.value.split("-")
  const year = Number(yearText)
  const month = Number(monthText)
  const day = Number(dayText)
  const maximum = daysInMonth(year, month)
  if (maximum === null) throw new Error("Cannot shift an invalid civil date")
  if (offset === 1 && day < maximum) return makeDate(year, month, day + 1)
  if (offset === -1 && day > 1) return makeDate(year, month, day - 1)
  if (offset === -1 && month === 1) return makeDate(year - 1, 12, 31)
  if (offset === 1 && month === 12) return makeDate(year + 1, 1, 1)
  const nextMonth = month + offset
  const nextMaximum = daysInMonth(year, nextMonth)
  if (nextMaximum === null) throw new Error("Cannot shift an invalid civil date")
  return makeDate(year, nextMonth, offset === 1 ? 1 : nextMaximum)
}

function makeDate(year: number, month: number, day: number): CivilDate {
  return {
    value: `${String(year).padStart(4, "0")}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`,
  }
}
