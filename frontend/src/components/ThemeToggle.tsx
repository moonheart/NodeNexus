import { Moon, Sun, Droplets } from "lucide-react"
import { useTheme } from "./ThemeProvider"
import { Button } from "./ui/button"

export function ThemeToggle() {
  const { setTheme } = useTheme()

  return (
    <div className="flex items-center gap-2">
      <Button variant="outline" size="icon" onClick={() => setTheme("light")}>
        <Sun className="h-[1.2rem] w-[1.2rem]" />
        <span className="sr-only">Switch to light theme</span>
      </Button>
      <Button variant="outline" size="icon" onClick={() => setTheme("dark")}>
        <Moon className="h-[1.2rem] w-[1.2rem]" />
        <span className="sr-only">Switch to dark theme</span>
      </Button>
      <Button variant="outline" size="icon" onClick={() => setTheme("ocean")}>
        <Droplets className="h-[1.2rem] w-[1.2rem]" />
        <span className="sr-only">Switch to ocean theme</span>
      </Button>
    </div>
  )
}