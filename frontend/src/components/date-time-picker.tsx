"use client"

import * as React from "react"
import { format, type Locale } from "date-fns"
import { Calendar as CalendarIcon } from "lucide-react"
import { useTranslation } from "react-i18next"
import { enUS, zhCN } from "date-fns/locale"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Calendar } from "@/components/ui/calendar"
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover"
import { Input } from "./ui/input"

interface DateTimePickerProps {
  value: Date | null;
  onChange: (date: Date | null) => void;
}

const localeMap: { [key: string]: Locale } = {
  en: enUS,
  "zh-CN": zhCN,
};

export function DateTimePicker({ value, onChange }: DateTimePickerProps) {
  const { t, i18n } = useTranslation();
  const [date, setDate] = React.useState<Date | undefined>(value || undefined);
  const [time, setTime] = React.useState(value ? format(value, "HH:mm:ss") : "00:00:00");

  const currentLocale = localeMap[i18n.language] || enUS;

  React.useEffect(() => {
    if (date) {
      const [hours, minutes, seconds] = time.split(':').map(Number);
      const newDate = new Date(date);
      newDate.setHours(hours, minutes, seconds);
      onChange(newDate);
    } else {
      onChange(null);
    }
  }, [date, time, onChange]);

  const handleDateSelect = (selectedDate: Date | undefined) => {
    if (selectedDate) {
        const newDate = new Date(selectedDate);
        if (time) {
            const [hours, minutes, seconds] = time.split(':').map(Number);
            newDate.setHours(hours, minutes, seconds);
        }
        setDate(newDate);
        onChange(newDate);
    } else {
        setDate(undefined);
        onChange(null);
    }
  }

  return (
    <div className="flex items-center gap-2">
      <Popover>
        <PopoverTrigger asChild>
          <Button
            variant={"outline"}
            className={cn(
              "w-[280px] justify-start text-left font-normal",
              !date && "text-muted-foreground"
            )}
          >
            <CalendarIcon className="mr-2 h-4 w-4" />
            {date ? format(date, "PPP", { locale: currentLocale }) : <span>{t('common.placeholders.pickDate')}</span>}
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-0">
          <Calendar
            mode="single"
            selected={date}
            onSelect={handleDateSelect}
            locale={currentLocale}
          />
        </PopoverContent>
      </Popover>
      <Input
        type="time"
        step="1"
        value={time}
        onChange={(e) => setTime(e.target.value)}
        className="w-[120px]"
      />
    </div>
  )
}