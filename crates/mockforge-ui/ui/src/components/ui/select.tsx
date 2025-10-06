import * as React from "react"
import { Check, ChevronDown } from "lucide-react"
import { cn } from "../../utils/cn"

// Fallback implementation without @radix-ui/react-select
interface SelectProps {
  children: React.ReactNode;
  value?: string;
  onValueChange?: (value: string) => void;
  defaultValue?: string;
  id?: string;
}

const SelectContext = React.createContext<{
  value: string;
  onValueChange: (value: string) => void;
  id?: string;
  options: Array<{ value: string; label: string }>;
  addOption: (value: string, label: string) => void;
} | null>(null);

const Select = ({ children, value, onValueChange, defaultValue, id }: SelectProps) => {
  const [internalValue, setInternalValue] = React.useState(defaultValue || value || '');
  const [options, setOptions] = React.useState<Array<{ value: string; label: string }>>([]);
  const currentValue = value || internalValue;

  const handleValueChange = (newValue: string) => {
    setInternalValue(newValue);
    onValueChange?.(newValue);
  };

  const addOption = React.useCallback((optValue: string, label: string) => {
    setOptions(prev => {
      if (prev.some(o => o.value === optValue)) return prev;
      return [...prev, { value: optValue, label }];
    });
  }, []);

  return (
    <SelectContext.Provider value={{ value: currentValue, onValueChange: handleValueChange, id, options, addOption }}>
      {children}
    </SelectContext.Provider>
  );
};

const SelectGroup = ({ children }: { children: React.ReactNode }) => <div>{children}</div>;

const SelectValue = ({ placeholder, children }: { placeholder?: string; children?: React.ReactNode }) => {
  const context = React.useContext(SelectContext);
  return <span>{children || context?.value || placeholder}</span>;
};

const SelectTrigger = React.forwardRef<
  HTMLSelectElement,
  React.SelectHTMLAttributes<HTMLSelectElement> & {
    children?: React.ReactNode;
    className?: string;
  }
>(({ className, children: _children, ...props }, ref) => {
  const context = React.useContext(SelectContext);

  // Render as a native select for testing compatibility
  return (
    <select
      ref={ref}
      id={context?.id}
      role="combobox"
      value={context?.value}
      onChange={(e) => context?.onValueChange(e.target.value)}
      className={cn(
        "flex h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 [&>span]:line-clamp-1",
        className
      )}
      {...props}
    >
      {context?.options.map(opt => (
        <option key={opt.value} value={opt.value}>{opt.label}</option>
      ))}
    </select>
  );
});
SelectTrigger.displayName = "SelectTrigger";

const SelectContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & { position?: 'popper' | 'item-aligned' }
>(({ className, children, position: _position = "popper", ...props }, ref) => (
  <div
    ref={ref}
    data-select-content
    className={cn(
      "relative z-50 max-h-96 min-w-[8rem] overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md p-1 hidden",
      className
    )}
    {...props}
  >
    {children}
  </div>
));
SelectContent.displayName = "SelectContent";

const SelectLabel = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("py-1.5 pl-8 pr-2 text-sm font-semibold", className)}
    {...props}
  />
));
SelectLabel.displayName = "SelectLabel";

const SelectItem = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & { value: string }
>(({ className, children, value, ...props }, ref) => {
  const context = React.useContext(SelectContext);

  React.useEffect(() => {
    if (context && typeof children === 'string') {
      context.addOption(value, children);
    }
  }, [context, value, children]);

  return (
    <div
      ref={ref}
      className={cn(
        "relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-8 pr-2 text-sm outline-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
        className
      )}
      data-value={value}
      {...props}
    >
      <span className="absolute left-2 flex h-3.5 w-3.5 items-center justify-center">
        <Check className="h-4 w-4" />
      </span>
      {children}
    </div>
  );
});
SelectItem.displayName = "SelectItem";

const SelectSeparator = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("-mx-1 my-1 h-px bg-muted", className)}
    {...props}
  />
));
SelectSeparator.displayName = "SelectSeparator";

const SelectScrollUpButton = ({ className: _className }: { className?: string }) => null;
const SelectScrollDownButton = ({ className: _className }: { className?: string }) => null;

export {
  Select,
  SelectGroup,
  SelectValue,
  SelectTrigger,
  SelectContent,
  SelectLabel,
  SelectItem,
  SelectSeparator,
  SelectScrollUpButton,
  SelectScrollDownButton,
}
