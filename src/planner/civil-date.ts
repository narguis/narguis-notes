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

export function shiftCivilDate(date: CivilDate, offset: number): CivilDate {
  const [yearText, monthText, dayText] = date.value.split("-")
  const year = Number(yearText)
  const month = Number(monthText)
  const day = Number(dayText)
  const maximum = daysInMonth(year, month)
  if (maximum === null) throw new Error("Cannot shift an invalid civil date")
  let result = { year, month, day }
  const step = offset < 0 ? -1 : 1
  for (let index = 0; index < Math.abs(offset); index += 1) {
    if (step === 1 && result.day < (daysInMonth(result.year, result.month) ?? 0)) result.day += 1
    else if (step === -1 && result.day > 1) result.day -= 1
    else if (step === -1 && result.month === 1) {
      result = { year: result.year - 1, month: 12, day: 31 }
    } else if (step === 1 && result.month === 12) {
      result = { year: result.year + 1, month: 1, day: 1 }
    } else {
      result.month += step
      result.day = step === 1 ? 1 : (daysInMonth(result.year, result.month) ?? 1)
    }
  }
  return makeDate(result.year, result.month, result.day)
}

export function civilWeekday(date: CivilDate): number {
  const parts = date.value.split("-").map(Number)
  const year = parts[0] ?? 0
  const month = parts[1] ?? 0
  const day = parts[2] ?? 0
  return new Date(Date.UTC(year, month - 1, day)).getUTCDay()
}

export function startOfWeek(date: CivilDate): CivilDate {
  return shiftCivilDate(date, -civilWeekday(date))
}

function makeDate(year: number, month: number, day: number): CivilDate {
  return {
    value: `${String(year).padStart(4, "0")}-${String(month).padStart(2, "0")}-${String(day).padStart(2, "0")}`,
  }
}
