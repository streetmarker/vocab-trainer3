/**
 * Formats a review date into a human-friendly Polish string.
 * Examples: 
 * - "Jutro 14:30"
 * - "Pn, 08:00"
 * - "za 15 min"
 */
export function formatReviewDate(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();
  const diffMin = Math.round(diffMs / 60000);

  // 1. Very soon
  if (diffMin < 1) return "teraz";
  if (diffMin < 60) return `za ${diffMin} min`;
  
  // 2. Today
  const isToday = date.toDateString() === now.toDateString();
  const timeStr = date.toLocaleTimeString('pl-PL', { hour: '2-digit', minute: '2-digit' });
  
  if (isToday) {
    return `dzisiaj ${timeStr}`;
  }

  // 3. Tomorrow
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);
  const isTomorrow = date.toDateString() === tomorrow.toDateString();
  
  if (isTomorrow) {
    return `jutro ${timeStr}`;
  }

  // 4. This week (show day name)
  const nextWeek = new Date(now);
  nextWeek.setDate(now.getDate() + 7);
  
  if (date < nextWeek) {
    const dayName = date.toLocaleDateString('pl-PL', { weekday: 'short' });
    return `${dayName.charAt(0).toUpperCase() + dayName.slice(1)}, ${timeStr}`;
  }

  // 5. Future (show full date)
  return date.toLocaleDateString('pl-PL', { day: 'numeric', month: 'short' });
}
