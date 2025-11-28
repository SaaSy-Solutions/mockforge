import * as React from "react";
import { cn } from "../../utils/cn";

export interface SliderProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type" | "onChange"> {
  /**
   * Minimum value for the slider
   */
  min?: number;
  /**
   * Maximum value for the slider
   */
  max?: number;
  /**
   * Step value for the slider
   */
  step?: number;
  /**
   * Current value of the slider
   */
  value?: number;
  /**
   * Callback fired when the value changes
   */
  onChange?: (value: number) => void;
  /**
   * Optional unit to display after the value (e.g., "ms", "%", "bps")
   */
  unit?: string;
  /**
   * Optional label to display above the slider
   */
  label?: string;
  /**
   * Whether to show the current value
   */
  showValue?: boolean;
  /**
   * Optional description text below the slider
   */
  description?: string;
}

/**
 * Slider component for numeric input with range control
 *
 * Provides an accessible range input with value display and optional unit formatting.
 * Matches the existing design system styling.
 */
const Slider = React.forwardRef<HTMLInputElement, SliderProps>(
  (
    {
      className,
      min = 0,
      max = 100,
      step = 1,
      value,
      onChange,
      unit,
      label,
      showValue = true,
      description,
      disabled,
      ...props
    },
    ref
  ) => {
    const [internalValue, setInternalValue] = React.useState(value ?? min);

    // Sync internal value with prop value
    React.useEffect(() => {
      if (value !== undefined) {
        setInternalValue(value);
      }
    }, [value]);

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      const newValue = parseFloat(e.target.value);
      setInternalValue(newValue);
      onChange?.(newValue);
    };

    const displayValue = value !== undefined ? value : internalValue;
    const percentage = ((displayValue - min) / (max - min)) * 100;

    return (
      <div className="w-full space-y-2">
        {(label || showValue) && (
          <div className="flex items-center justify-between">
            {label && (
              <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                {label}
              </label>
            )}
            {showValue && (
              <span className="text-sm font-semibold text-gray-900 dark:text-gray-100 tabular-nums">
                {displayValue.toLocaleString()}
                {unit && <span className="ml-1 text-gray-500 dark:text-gray-400">{unit}</span>}
              </span>
            )}
          </div>
        )}
        <div className="relative flex items-center">
          <input
            type="range"
            ref={ref}
            min={min}
            max={max}
            step={step}
            value={displayValue}
            onChange={handleChange}
            disabled={disabled}
            className={cn(
              "h-2 w-full appearance-none rounded-lg bg-gray-200 dark:bg-gray-700 outline-none transition-all",
              "disabled:opacity-50 disabled:cursor-not-allowed",
              // Webkit (Chrome, Safari, Edge)
              "[&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-blue-600 dark:[&::-webkit-slider-thumb]:bg-blue-500 [&::-webkit-slider-thumb]:cursor-pointer [&::-webkit-slider-thumb]:shadow-sm [&::-webkit-slider-thumb]:transition-all [&::-webkit-slider-thumb]:hover:bg-blue-700 dark:[&::-webkit-slider-thumb]:hover:bg-blue-400 [&::-webkit-slider-thumb]:active:scale-110",
              // Firefox
              "[&::-moz-range-thumb]:h-4 [&::-moz-range-thumb]:w-4 [&::-moz-range-thumb]:rounded-full [&::-moz-range-thumb]:bg-blue-600 dark:[&::-moz-range-thumb]:bg-blue-500 [&::-moz-range-thumb]:border-0 [&::-moz-range-thumb]:cursor-pointer [&::-moz-range-thumb]:shadow-sm [&::-moz-range-thumb]:transition-all [&::-moz-range-thumb]:hover:bg-blue-700 dark:[&::-moz-range-thumb]:hover:bg-blue-400",
              // Track fill (visual progress indicator)
              "before:absolute before:left-0 before:top-0 before:h-2 before:rounded-lg before:bg-blue-600 dark:before:bg-blue-500 before:pointer-events-none",
              className
            )}
            style={{
              // @ts-ignore - CSS custom property for track fill
              "--track-fill": `${percentage}%`,
              background: `linear-gradient(to right, rgb(37 99 235) 0%, rgb(37 99 235) var(--track-fill), rgb(229 231 235) var(--track-fill), rgb(229 231 235) 100%)`,
            }}
            {...props}
          />
        </div>
        {description && (
          <p className="text-xs text-gray-500 dark:text-gray-400">{description}</p>
        )}
        {/* Min/Max labels */}
        <div className="flex items-center justify-between text-xs text-gray-400 dark:text-gray-500">
          <span>{min}{unit && ` ${unit}`}</span>
          <span>{max}{unit && ` ${unit}`}</span>
        </div>
      </div>
    );
  }
);

Slider.displayName = "Slider";

export { Slider };
