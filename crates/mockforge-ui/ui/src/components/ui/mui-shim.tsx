/* eslint-disable @typescript-eslint/no-explicit-any, react-refresh/only-export-components */
/**
 * Compatibility shim for `@mui/material`.
 *
 * Re-implements the MUI components that legacy pages still import, using
 * shadcn/ui + Tailwind tokens. The point is to drop the @mui/material
 * dependency without rewriting every page in one PR.
 *
 * This is intentionally a *compatibility surface*, not a faithful MUI
 * reproduction:
 *   - The `sx` prop accepts a small subset (spacing, display, gap, color
 *     keywords). Complex theme breakpoint queries fall through to inline
 *     style.
 *   - Variants/colors map to the closest semantic token.
 *   - Behaviour (open/close, controlled inputs) is preserved.
 *
 * If a behaviour is missing, fix it here rather than reaching back to
 * @mui/material — that package should not be reinstalled.
 */
import * as React from 'react';
import { cn } from '../../utils/cn';
import { X, ChevronDown } from 'lucide-react';

// -----------------------------------------------------------------------------
// sx prop translation
// -----------------------------------------------------------------------------

const SPACING_UNIT = 8; // MUI default

function spacing(v: number | string | undefined): string | undefined {
  if (v === undefined) return undefined;
  if (typeof v === 'number') return `${v * SPACING_UNIT}px`;
  return v;
}

const COLOR_KEYWORDS: Record<string, string> = {
  'primary.main': 'hsl(var(--primary))',
  'primary.light': 'hsl(var(--primary) / 0.8)',
  'primary.dark': 'hsl(var(--primary) / 1.1)',
  'secondary.main': 'hsl(var(--secondary))',
  'error.main': 'hsl(var(--destructive))',
  'error.light': 'hsl(var(--destructive) / 0.5)',
  'warning.main': 'hsl(var(--warning))',
  'info.main': 'hsl(var(--info))',
  'success.main': 'hsl(var(--success))',
  'text.primary': 'hsl(var(--foreground))',
  'text.secondary': 'hsl(var(--muted-foreground))',
  'text.disabled': 'hsl(var(--muted-foreground) / 0.6)',
  'background.paper': 'hsl(var(--card))',
  'background.default': 'hsl(var(--background))',
  'divider': 'hsl(var(--border))',
  'action.hover': 'hsl(var(--accent))',
  'action.selected': 'hsl(var(--accent))',
  'grey.50': 'hsl(var(--muted))',
  'grey.100': 'hsl(var(--muted))',
  'grey.200': 'hsl(var(--border))',
  'grey.300': 'hsl(var(--border))',
};

function colorVal(c: unknown): string | undefined {
  if (typeof c !== 'string') return undefined;
  return COLOR_KEYWORDS[c] ?? c;
}

type SxValue = string | number | undefined;
interface SxObject {
  p?: SxValue; pt?: SxValue; pr?: SxValue; pb?: SxValue; pl?: SxValue;
  px?: SxValue; py?: SxValue;
  m?: SxValue; mt?: SxValue; mr?: SxValue; mb?: SxValue; ml?: SxValue;
  mx?: SxValue; my?: SxValue;
  width?: SxValue; minWidth?: SxValue; maxWidth?: SxValue;
  height?: SxValue; minHeight?: SxValue; maxHeight?: SxValue;
  display?: string;
  flexDirection?: string;
  alignItems?: string;
  justifyContent?: string;
  flexWrap?: string;
  flex?: string | number;
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: SxValue;
  gap?: SxValue;
  rowGap?: SxValue;
  columnGap?: SxValue;
  color?: string;
  bgcolor?: string;
  backgroundColor?: string;
  border?: string | number;
  borderColor?: string;
  borderRadius?: SxValue;
  borderTop?: string | number;
  borderBottom?: string | number;
  borderLeft?: string | number;
  borderRight?: string | number;
  fontSize?: SxValue;
  fontWeight?: string | number;
  fontFamily?: string;
  textAlign?: string;
  textTransform?: string;
  lineHeight?: SxValue;
  letterSpacing?: SxValue;
  whiteSpace?: string;
  overflow?: string;
  overflowX?: string;
  overflowY?: string;
  textOverflow?: string;
  position?: string;
  top?: SxValue; right?: SxValue; bottom?: SxValue; left?: SxValue;
  zIndex?: number | string;
  cursor?: string;
  opacity?: number;
  boxShadow?: string | number;
  transition?: string;
  [key: string]: unknown;
}

function sxToStyle(sx: SxObject | undefined | null): React.CSSProperties | undefined {
  if (!sx) return undefined;
  const s: React.CSSProperties = {};
  const setSpacing = (key: keyof React.CSSProperties, v: SxValue) => {
    if (v === undefined) return;
    (s as Record<string, string | undefined>)[key as string] = spacing(v);
  };

  setSpacing('padding', sx.p);
  setSpacing('paddingTop', sx.pt ?? sx.py);
  setSpacing('paddingRight', sx.pr ?? sx.px);
  setSpacing('paddingBottom', sx.pb ?? sx.py);
  setSpacing('paddingLeft', sx.pl ?? sx.px);
  setSpacing('margin', sx.m);
  setSpacing('marginTop', sx.mt ?? sx.my);
  setSpacing('marginRight', sx.mr ?? sx.mx);
  setSpacing('marginBottom', sx.mb ?? sx.my);
  setSpacing('marginLeft', sx.ml ?? sx.mx);
  setSpacing('width', sx.width);
  setSpacing('minWidth', sx.minWidth);
  setSpacing('maxWidth', sx.maxWidth);
  setSpacing('height', sx.height);
  setSpacing('minHeight', sx.minHeight);
  setSpacing('maxHeight', sx.maxHeight);
  setSpacing('gap', sx.gap);
  setSpacing('rowGap', sx.rowGap);
  setSpacing('columnGap', sx.columnGap);
  setSpacing('top', sx.top);
  setSpacing('right', sx.right);
  setSpacing('bottom', sx.bottom);
  setSpacing('left', sx.left);
  setSpacing('fontSize', sx.fontSize);
  setSpacing('lineHeight', sx.lineHeight);
  setSpacing('letterSpacing', sx.letterSpacing);
  setSpacing('borderRadius', sx.borderRadius);

  if (sx.display) s.display = sx.display;
  if (sx.flexDirection) s.flexDirection = sx.flexDirection as React.CSSProperties['flexDirection'];
  if (sx.alignItems) s.alignItems = sx.alignItems as React.CSSProperties['alignItems'];
  if (sx.justifyContent) s.justifyContent = sx.justifyContent as React.CSSProperties['justifyContent'];
  if (sx.flexWrap) s.flexWrap = sx.flexWrap as React.CSSProperties['flexWrap'];
  if (sx.flex !== undefined) s.flex = sx.flex;
  if (sx.flexGrow !== undefined) s.flexGrow = sx.flexGrow;
  if (sx.flexShrink !== undefined) s.flexShrink = sx.flexShrink;
  if (sx.flexBasis !== undefined) s.flexBasis = spacing(sx.flexBasis);
  if (sx.color) s.color = colorVal(sx.color);
  if (sx.bgcolor) s.backgroundColor = colorVal(sx.bgcolor);
  if (sx.backgroundColor) s.backgroundColor = colorVal(sx.backgroundColor);
  if (sx.borderColor) s.borderColor = colorVal(sx.borderColor);
  if (sx.border !== undefined) {
    s.border = typeof sx.border === 'number' ? `${sx.border}px solid hsl(var(--border))` : (sx.border as string);
  }
  if (sx.fontWeight !== undefined) s.fontWeight = sx.fontWeight;
  if (sx.fontFamily) s.fontFamily = sx.fontFamily;
  if (sx.textAlign) s.textAlign = sx.textAlign as React.CSSProperties['textAlign'];
  if (sx.textTransform) s.textTransform = sx.textTransform as React.CSSProperties['textTransform'];
  if (sx.whiteSpace) s.whiteSpace = sx.whiteSpace as React.CSSProperties['whiteSpace'];
  if (sx.overflow) s.overflow = sx.overflow as React.CSSProperties['overflow'];
  if (sx.overflowX) s.overflowX = sx.overflowX as React.CSSProperties['overflowX'];
  if (sx.overflowY) s.overflowY = sx.overflowY as React.CSSProperties['overflowY'];
  if (sx.textOverflow) s.textOverflow = sx.textOverflow;
  if (sx.position) s.position = sx.position as React.CSSProperties['position'];
  if (sx.zIndex !== undefined) s.zIndex = sx.zIndex as React.CSSProperties['zIndex'];
  if (sx.cursor) s.cursor = sx.cursor;
  if (sx.opacity !== undefined) s.opacity = sx.opacity;
  if (sx.boxShadow !== undefined) s.boxShadow = sx.boxShadow as string;
  if (sx.transition) s.transition = sx.transition;

  return s;
}

// -----------------------------------------------------------------------------
// Box / Stack / Container / Paper / Divider
// -----------------------------------------------------------------------------

interface BoxProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  sx?: SxObject;
  component?: keyof React.JSX.IntrinsicElements | React.ComponentType<any>;
}

export const Box = React.forwardRef<HTMLDivElement, BoxProps>(function Box(
  { sx, component, style, className, children, ...rest }, ref
) {
  const Comp = (component ?? 'div') as any;
  return (
    <Comp ref={ref} className={className} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </Comp>
  );
});

interface StackProps extends BoxProps {
  direction?: 'row' | 'column' | 'row-reverse' | 'column-reverse';
  spacing?: number;
  alignItems?: string;
  justifyContent?: string;
  divider?: React.ReactNode;
}

export const Stack = React.forwardRef<HTMLDivElement, StackProps>(function Stack(
  { direction = 'column', spacing: stackSpacing, alignItems, justifyContent, divider, sx, style, className, children, ...rest }, ref
) {
  const styleObj: React.CSSProperties = {
    display: 'flex',
    flexDirection: direction,
    gap: stackSpacing ? `${stackSpacing * SPACING_UNIT}px` : undefined,
    alignItems,
    justifyContent,
    ...sxToStyle(sx),
    ...style,
  };
  const items = React.Children.toArray(children);
  const withDivs = divider
    ? items.flatMap((c, i) => (i === 0 ? [c] : [<React.Fragment key={`d${i}`}>{divider}</React.Fragment>, c]))
    : items;
  return (
    <div ref={ref} className={className} style={styleObj} {...rest}>
      {withDivs}
    </div>
  );
});

interface ContainerProps extends BoxProps {
  maxWidth?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | false;
  fixed?: boolean;
  disableGutters?: boolean;
}

export function Container({ maxWidth = 'lg', disableGutters, className, children, sx, style, ...rest }: ContainerProps) {
  const widths = {
    xs: '444px', sm: '600px', md: '900px', lg: '1200px', xl: '1536px',
  };
  return (
    <div
      className={cn('mx-auto w-full', !disableGutters && 'px-4 md:px-6', className)}
      style={{ maxWidth: maxWidth ? widths[maxWidth] : undefined, ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </div>
  );
}

interface PaperProps extends BoxProps {
  elevation?: number;
  variant?: 'elevation' | 'outlined';
  square?: boolean;
}

export const Paper = React.forwardRef<HTMLDivElement, PaperProps>(function Paper(
  { elevation = 1, variant, square, className, children, sx, style, ...rest }, ref
) {
  const shadow =
    variant === 'outlined' ? '' :
    elevation === 0 ? '' :
    elevation <= 2 ? 'shadow-sm' :
    elevation <= 6 ? 'shadow-md' :
    elevation <= 12 ? 'shadow-lg' : 'shadow-xl';
  return (
    <div
      ref={ref}
      className={cn(
        'bg-card text-card-foreground',
        variant === 'outlined' ? 'border border-border' : '',
        shadow,
        !square && 'rounded-lg',
        className
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </div>
  );
});

interface DividerProps extends Omit<React.HTMLAttributes<HTMLHRElement | HTMLDivElement>, 'children'> {
  orientation?: 'horizontal' | 'vertical';
  flexItem?: boolean;
  textAlign?: 'left' | 'center' | 'right';
  variant?: 'fullWidth' | 'inset' | 'middle';
  sx?: SxObject;
  children?: React.ReactNode;
}

export function Divider({ orientation = 'horizontal', flexItem, textAlign, className, children, sx, style, ...rest }: DividerProps) {
  if (children) {
    return (
      <div
        role="separator"
        className={cn('flex items-center gap-3 text-xs text-muted-foreground my-2', textAlign === 'left' && 'flex-row', textAlign === 'right' && 'flex-row-reverse', className)}
        style={{ ...sxToStyle(sx), ...style }}
      >
        <span className="flex-1 h-px bg-border" />
        <span>{children}</span>
        <span className="flex-1 h-px bg-border" />
      </div>
    );
  }
  if (orientation === 'vertical') {
    return <div role="separator" className={cn('w-px self-stretch bg-border', flexItem && 'h-auto', className)} style={{ ...sxToStyle(sx), ...style }} {...(rest as React.HTMLAttributes<HTMLDivElement>)} />;
  }
  return <hr className={cn('border-0 h-px bg-border my-0', className)} style={{ ...sxToStyle(sx), ...style }} {...(rest as React.HTMLAttributes<HTMLHRElement>)} />;
}

// -----------------------------------------------------------------------------
// Typography
// -----------------------------------------------------------------------------

type TypoVariant = 'h1' | 'h2' | 'h3' | 'h4' | 'h5' | 'h6'
  | 'subtitle1' | 'subtitle2' | 'body1' | 'body2'
  | 'button' | 'caption' | 'overline' | 'inherit';

interface TypographyProps extends React.HTMLAttributes<HTMLElement> {
  variant?: TypoVariant;
  component?: keyof React.JSX.IntrinsicElements;
  color?: string;
  align?: 'inherit' | 'left' | 'center' | 'right' | 'justify';
  gutterBottom?: boolean;
  noWrap?: boolean;
  paragraph?: boolean;
  sx?: SxObject;
}

const TYPO_TAG: Record<TypoVariant, keyof React.JSX.IntrinsicElements> = {
  h1: 'h1', h2: 'h2', h3: 'h3', h4: 'h4', h5: 'h5', h6: 'h6',
  subtitle1: 'h6', subtitle2: 'h6',
  body1: 'p', body2: 'p',
  button: 'span', caption: 'span', overline: 'span', inherit: 'span',
};

const TYPO_CLASS: Record<TypoVariant, string> = {
  h1: 'text-5xl font-bold tracking-tight',
  h2: 'text-4xl font-bold tracking-tight',
  h3: 'text-3xl font-bold tracking-tight',
  h4: 'text-2xl font-semibold',
  h5: 'text-xl font-semibold',
  h6: 'text-lg font-semibold',
  subtitle1: 'text-base font-medium',
  subtitle2: 'text-sm font-medium',
  body1: 'text-base',
  body2: 'text-sm',
  button: 'text-sm font-medium uppercase tracking-wider',
  caption: 'text-xs text-muted-foreground',
  overline: 'text-xs uppercase tracking-widest text-muted-foreground',
  inherit: '',
};

export function Typography({
  variant = 'body1',
  component,
  color,
  align,
  gutterBottom,
  noWrap,
  paragraph,
  className,
  sx,
  style,
  children,
  ...rest
}: TypographyProps) {
  const Tag = (component ?? (paragraph ? 'p' : TYPO_TAG[variant])) as React.ElementType;
  const colorClass =
    color === 'primary' ? 'text-primary' :
    color === 'secondary' ? 'text-secondary' :
    color === 'error' || color === 'error.main' ? 'text-destructive' :
    color === 'warning' || color === 'warning.main' ? 'text-warning' :
    color === 'info' || color === 'info.main' ? 'text-info' :
    color === 'success' || color === 'success.main' ? 'text-success' :
    color === 'text.primary' ? 'text-foreground' :
    color === 'text.secondary' ? 'text-muted-foreground' :
    color === 'text.disabled' ? 'text-muted-foreground/60' :
    color === 'textSecondary' ? 'text-muted-foreground' :
    color === 'textPrimary' ? 'text-foreground' : '';
  const inlineColor = !colorClass && color ? colorVal(color) : undefined;
  return (
    <Tag
      className={cn(
        TYPO_CLASS[variant],
        colorClass,
        align && `text-${align}`,
        gutterBottom && 'mb-2',
        paragraph && 'mb-4',
        noWrap && 'truncate',
        className
      )}
      style={{ color: inlineColor, ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </Tag>
  );
}

// -----------------------------------------------------------------------------
// Button / IconButton
// -----------------------------------------------------------------------------

interface MuiButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'color'> {
  variant?: 'text' | 'contained' | 'outlined';
  color?: 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success' | 'inherit';
  size?: 'small' | 'medium' | 'large';
  disabled?: boolean;
  fullWidth?: boolean;
  startIcon?: React.ReactNode;
  endIcon?: React.ReactNode;
  href?: string;
  target?: string;
  component?: React.ElementType;
  sx?: SxObject;
}

function buttonColorClasses(variant: string, color: string): string {
  const map: Record<string, Record<string, string>> = {
    contained: {
      primary: 'bg-primary text-primary-foreground hover:bg-primary/90',
      secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/80',
      error: 'bg-destructive text-destructive-foreground hover:bg-destructive/90',
      warning: 'bg-warning text-text-inverse hover:bg-warning-600',
      info: 'bg-info text-text-inverse hover:bg-info-600',
      success: 'bg-success text-text-inverse hover:bg-success-600',
      inherit: 'bg-muted text-foreground hover:bg-muted/80',
    },
    outlined: {
      primary: 'border border-primary text-primary hover:bg-primary/10',
      secondary: 'border border-secondary text-secondary hover:bg-secondary/10',
      error: 'border border-destructive text-destructive hover:bg-destructive/10',
      warning: 'border border-warning text-warning hover:bg-warning/10',
      info: 'border border-info text-info hover:bg-info/10',
      success: 'border border-success text-success hover:bg-success/10',
      inherit: 'border border-border text-foreground hover:bg-accent hover:text-accent-foreground',
    },
    text: {
      primary: 'text-primary hover:bg-primary/10',
      secondary: 'text-secondary hover:bg-secondary/10',
      error: 'text-destructive hover:bg-destructive/10',
      warning: 'text-warning hover:bg-warning/10',
      info: 'text-info hover:bg-info/10',
      success: 'text-success hover:bg-success/10',
      inherit: 'text-foreground hover:bg-accent hover:text-accent-foreground',
    },
  };
  return map[variant]?.[color] ?? map.text.primary;
}

export const Button = React.forwardRef<HTMLButtonElement, MuiButtonProps>(function Button(
  {
    variant = 'text',
    color = 'primary',
    size = 'medium',
    disabled,
    fullWidth,
    startIcon,
    endIcon,
    component,
    className,
    children,
    sx,
    style,
    ...rest
  }, ref
) {
  const sizes = {
    small: 'px-3 py-1 text-xs',
    medium: 'px-4 py-2 text-sm',
    large: 'px-6 py-3 text-base',
  };
  const Comp = (component ?? (rest.href ? 'a' : 'button')) as React.ElementType;
  return (
    <Comp
      ref={ref}
      disabled={disabled}
      className={cn(
        'inline-flex items-center justify-center gap-2 rounded-md font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:opacity-50 disabled:pointer-events-none',
        buttonColorClasses(variant, color),
        sizes[size],
        fullWidth && 'w-full',
        className
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {startIcon}
      {children}
      {endIcon}
    </Comp>
  );
});

interface IconButtonProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'color'> {
  color?: 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success' | 'inherit' | 'default';
  size?: 'small' | 'medium' | 'large';
  edge?: 'start' | 'end' | false;
  sx?: SxObject;
}

export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { color = 'default', size = 'medium', className, children, sx, style, ...rest }, ref
) {
  const sizes = { small: 'h-8 w-8', medium: 'h-10 w-10', large: 'h-12 w-12' };
  const colors: Record<string, string> = {
    primary: 'text-primary hover:bg-primary/10',
    secondary: 'text-secondary hover:bg-secondary/10',
    error: 'text-destructive hover:bg-destructive/10',
    warning: 'text-warning hover:bg-warning/10',
    info: 'text-info hover:bg-info/10',
    success: 'text-success hover:bg-success/10',
    inherit: 'text-current hover:bg-accent',
    default: 'text-muted-foreground hover:bg-accent hover:text-accent-foreground',
  };
  return (
    <button
      ref={ref}
      className={cn(
        'inline-flex items-center justify-center rounded-full transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:opacity-50 disabled:pointer-events-none',
        sizes[size],
        colors[color],
        className
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </button>
  );
});

// -----------------------------------------------------------------------------
// Grid
// -----------------------------------------------------------------------------

type GridSize = number | 'auto' | true | false;
interface GridProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  container?: boolean;
  item?: boolean;
  spacing?: number;
  rowSpacing?: number;
  columnSpacing?: number;
  xs?: GridSize; sm?: GridSize; md?: GridSize; lg?: GridSize; xl?: GridSize;
  alignItems?: string;
  justifyContent?: string;
  direction?: string;
  wrap?: 'wrap' | 'nowrap' | 'wrap-reverse';
  sx?: SxObject;
}

function gridSpan(s: GridSize | undefined): string | undefined {
  if (s === undefined) return undefined;
  if (s === true || s === 'auto') return 'auto';
  if (s === false) return undefined;
  return `span ${s} / span ${s}`;
}

export const Grid = React.forwardRef<HTMLDivElement, GridProps>(function Grid(
  { container, item, spacing: gridSpacing, rowSpacing, columnSpacing, xs, sm, md, lg, xl, alignItems, justifyContent, direction, wrap, className, children, sx, style, ...rest }, ref
) {
  if (container) {
    return (
      <div
        ref={ref}
        className={cn('grid grid-cols-12', wrap === 'nowrap' && 'flex-nowrap', className)}
        style={{
          gap: gridSpacing ? `${gridSpacing * SPACING_UNIT}px` : undefined,
          rowGap: rowSpacing ? `${rowSpacing * SPACING_UNIT}px` : undefined,
          columnGap: columnSpacing ? `${columnSpacing * SPACING_UNIT}px` : undefined,
          alignItems,
          justifyContent,
          ...(direction && { flexDirection: direction as React.CSSProperties['flexDirection'] }),
          ...sxToStyle(sx),
          ...style,
        }}
        {...rest}
      >
        {children}
      </div>
    );
  }

  // Items: Tailwind v4 can't JIT `col-span-${n}` from a string
  // interpolation, so the shim emits `--gs-xs/sm/md/lg/xl` custom
  // properties inline, and the matching `.mui-grid-item` rules in
  // index.css read them at each breakpoint — preserves MUI's responsive
  // Grid semantics without requiring Tailwind to enumerate the classes.
  const responsive: Record<string, string | undefined> = {
    '--gs-xs': gridSpan(xs),
    '--gs-sm': gridSpan(sm),
    '--gs-md': gridSpan(md),
    '--gs-lg': gridSpan(lg),
    '--gs-xl': gridSpan(xl),
  };
  const itemStyle: React.CSSProperties = {
    ...(responsive as React.CSSProperties),
    ...sxToStyle(sx),
    ...style,
  };
  const hasAnySpan = xs !== undefined || sm !== undefined || md !== undefined || lg !== undefined || xl !== undefined;

  if (item || hasAnySpan) {
    return (
      <div
        ref={ref}
        className={cn('mui-grid-item', className)}
        style={itemStyle}
        {...rest}
      >
        {children}
      </div>
    );
  }
  return (
    <div ref={ref} className={className} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
});

// -----------------------------------------------------------------------------
// Card / CardContent / CardActions / CardMedia
// -----------------------------------------------------------------------------

interface CardProps extends BoxProps {
  variant?: 'elevation' | 'outlined';
  raised?: boolean;
}

export const Card = React.forwardRef<HTMLDivElement, CardProps>(function Card(
  { variant, raised, className, children, sx, style, ...rest }, ref
) {
  return (
    <div
      ref={ref}
      className={cn(
        'bg-card text-card-foreground rounded-xl',
        variant === 'outlined' ? 'border border-border' : 'shadow-sm border border-border/50',
        raised && 'shadow-lg',
        className
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </div>
  );
});

export function CardContent({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('p-6', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function CardActions({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('flex items-center gap-2 px-4 py-2 border-t border-border/50', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

interface CardMediaProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  image?: string;
  src?: string;
  component?: React.ElementType;
  alt?: string;
  sx?: SxObject;
  height?: number | string;
}

export function CardMedia({ image, src, component, alt, height, className, children, sx, style, ...rest }: CardMediaProps) {
  if (component === 'img' || src) {
    return <img src={src ?? image} alt={alt} className={cn('w-full object-cover', className)} style={{ height, ...sxToStyle(sx), ...style }} {...rest as any} />;
  }
  return (
    <div
      className={cn('w-full bg-muted bg-center bg-cover', className)}
      style={{ height: height ?? 200, backgroundImage: image ? `url(${image})` : undefined, ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Dialog / DialogTitle / DialogContent / DialogContentText / DialogActions
// -----------------------------------------------------------------------------

interface DialogProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onClose'> {
  open: boolean;
  onClose?: (event?: object, reason?: string) => void;
  fullWidth?: boolean;
  maxWidth?: 'xs' | 'sm' | 'md' | 'lg' | 'xl' | false;
  fullScreen?: boolean;
  scroll?: 'paper' | 'body';
  sx?: SxObject;
  PaperProps?: { sx?: SxObject; className?: string };
}

export function Dialog({ open, onClose, fullWidth, maxWidth = 'sm', fullScreen, className, children, sx, style, PaperProps, ...rest }: DialogProps) {
  React.useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose?.(e, 'escapeKeyDown'); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, onClose]);

  if (!open) return null;
  const widths = { xs: '444px', sm: '600px', md: '900px', lg: '1200px', xl: '1536px' };
  return (
    <div
      role="presentation"
      className={cn('fixed inset-0 z-50 flex items-center justify-center p-4', className)}
      style={style}
      {...rest}
    >
      <div className="absolute inset-0 bg-bg-overlay" onClick={() => onClose?.({}, 'backdropClick')} />
      <div
        role="dialog"
        aria-modal="true"
        className={cn(
          'relative bg-card text-card-foreground rounded-lg shadow-xl border border-border max-h-[90vh] overflow-y-auto',
          fullScreen ? 'w-full h-full max-h-full rounded-none' : '',
          fullWidth && !fullScreen ? 'w-full' : '',
          PaperProps?.className,
        )}
        style={{
          maxWidth: fullScreen ? '100%' : (maxWidth ? widths[maxWidth] : undefined),
          ...sxToStyle(sx),
          ...sxToStyle(PaperProps?.sx),
        }}
      >
        {children}
      </div>
    </div>
  );
}

export function DialogTitle({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('px-6 pt-6 pb-3 text-lg font-semibold text-foreground', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function DialogContent({ className, children, sx, style, ...rest }: BoxProps & { dividers?: boolean }) {
  return (
    <div className={cn('px-6 py-3', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function DialogContentText({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <p className={cn('text-sm text-muted-foreground', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </p>
  );
}

export function DialogActions({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('flex items-center justify-end gap-2 px-6 py-4', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Drawer (modal sidebar)
// -----------------------------------------------------------------------------

interface DrawerProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onClose'> {
  open: boolean;
  onClose?: (event?: object, reason?: string) => void;
  anchor?: 'left' | 'right' | 'top' | 'bottom';
  variant?: 'temporary' | 'persistent' | 'permanent';
  PaperProps?: { sx?: SxObject; className?: string };
}

export function Drawer({ open, onClose, anchor = 'left', variant = 'temporary', className, children, PaperProps, ...rest }: DrawerProps) {
  if (!open && variant === 'temporary') return null;
  const side = {
    left: 'left-0 top-0 h-full w-80',
    right: 'right-0 top-0 h-full w-80',
    top: 'top-0 left-0 w-full h-1/2',
    bottom: 'bottom-0 left-0 w-full h-1/2',
  }[anchor];
  return (
    <div className="fixed inset-0 z-50" {...rest}>
      {variant === 'temporary' && (
        <div className="absolute inset-0 bg-bg-overlay" onClick={() => onClose?.({}, 'backdropClick')} />
      )}
      <div
        className={cn(
          'absolute bg-card text-card-foreground border-border shadow-xl',
          anchor === 'left' && 'border-r',
          anchor === 'right' && 'border-l',
          anchor === 'top' && 'border-b',
          anchor === 'bottom' && 'border-t',
          side,
          className,
          PaperProps?.className,
        )}
        style={sxToStyle(PaperProps?.sx)}
      >
        {children}
      </div>
    </div>
  );
}

// -----------------------------------------------------------------------------
// Form controls — TextField, Select, MenuItem, FormControl, InputLabel, ...
// -----------------------------------------------------------------------------

const inputBase =
  'w-full rounded-md border border-input bg-background text-foreground placeholder:text-muted-foreground ' +
  'focus:border-ring focus:outline-none focus:ring-2 focus:ring-ring/30 ' +
  'transition-colors disabled:opacity-50 disabled:cursor-not-allowed';

interface TextFieldProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size' | 'color'> {
  label?: React.ReactNode;
  helperText?: React.ReactNode;
  error?: boolean;
  variant?: 'standard' | 'outlined' | 'filled';
  size?: 'small' | 'medium';
  fullWidth?: boolean;
  multiline?: boolean;
  rows?: number;
  minRows?: number;
  maxRows?: number;
  margin?: 'none' | 'dense' | 'normal';
  select?: boolean;
  children?: React.ReactNode;
  InputProps?: { startAdornment?: React.ReactNode; endAdornment?: React.ReactNode; readOnly?: boolean };
  inputProps?: React.InputHTMLAttributes<HTMLInputElement>;
  SelectProps?: Record<string, unknown>;
  sx?: SxObject;
  color?: string;
}

export function TextField({
  label, helperText, error, size = 'medium', fullWidth, multiline, rows, minRows,
  margin, select, children, InputProps, inputProps,
  className, value, onChange, type, name, id, placeholder, disabled, required,
  sx, style, ...rest
}: TextFieldProps) {
  const sizeClass = size === 'small' ? 'px-2.5 py-1 text-sm' : 'px-3 py-2 text-sm';
  const wrapMargin = margin === 'dense' ? 'my-1' : margin === 'normal' ? 'my-3' : '';
  const errorClass = error ? 'border-destructive focus:border-destructive focus:ring-destructive/30' : '';
  const inputCls = cn(inputBase, sizeClass, errorClass, className);
  const fieldId = id ?? React.useId();

  return (
    <div className={cn(fullWidth && 'w-full', wrapMargin)} style={{ ...sxToStyle(sx), ...style }}>
      {label && (
        <label htmlFor={fieldId} className={cn('block text-sm font-medium mb-1', error ? 'text-destructive' : 'text-foreground')}>
          {label}{required && <span className="text-destructive ml-0.5">*</span>}
        </label>
      )}
      <div className="relative flex items-center">
        {InputProps?.startAdornment && <span className="absolute left-2 flex items-center text-muted-foreground">{InputProps.startAdornment}</span>}
        {select ? (
          <select
            id={fieldId}
            name={name}
            value={value as string | number | readonly string[] | undefined}
            onChange={onChange as any}
            disabled={disabled}
            required={required}
            className={cn(inputCls, 'cursor-pointer pr-8')}
          >
            {children}
          </select>
        ) : multiline ? (
          <textarea
            id={fieldId}
            name={name}
            value={value as string | number | readonly string[] | undefined}
            onChange={onChange as any}
            placeholder={placeholder}
            disabled={disabled}
            required={required}
            rows={rows ?? minRows ?? 3}
            className={cn(inputCls, 'resize-y', InputProps?.startAdornment && 'pl-8', InputProps?.endAdornment && 'pr-8')}
            {...(inputProps as any)}
          />
        ) : (
          <input
            id={fieldId}
            name={name}
            type={type}
            value={value as string | number | readonly string[] | undefined}
            onChange={onChange as any}
            placeholder={placeholder}
            disabled={disabled}
            required={required}
            readOnly={InputProps?.readOnly}
            className={cn(inputCls, InputProps?.startAdornment && 'pl-8', InputProps?.endAdornment && 'pr-8')}
            {...(inputProps as any)}
            {...rest}
          />
        )}
        {InputProps?.endAdornment && <span className="absolute right-2 flex items-center text-muted-foreground">{InputProps.endAdornment}</span>}
      </div>
      {helperText && (
        <p className={cn('mt-1 text-xs', error ? 'text-destructive' : 'text-muted-foreground')}>{helperText}</p>
      )}
    </div>
  );
}

interface FormControlProps extends BoxProps {
  fullWidth?: boolean;
  error?: boolean;
  required?: boolean;
  size?: 'small' | 'medium';
  variant?: 'standard' | 'outlined' | 'filled';
  margin?: 'none' | 'dense' | 'normal';
  disabled?: boolean;
}

export function FormControl({ fullWidth, className, children, sx, style, ...rest }: FormControlProps) {
  return (
    <div className={cn('flex flex-col', fullWidth && 'w-full', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

interface InputLabelProps extends React.LabelHTMLAttributes<HTMLLabelElement> {
  shrink?: boolean;
  error?: boolean;
  required?: boolean;
  sx?: SxObject;
}

export function InputLabel({ className, children, error, required, sx, style, shrink: _shrink, ...rest }: InputLabelProps) {
  return (
    <label
      className={cn('block text-sm font-medium mb-1', error ? 'text-destructive' : 'text-foreground', className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}{required && <span className="text-destructive ml-0.5">*</span>}
    </label>
  );
}

interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, 'size' | 'color' | 'onChange'> {
  size?: 'small' | 'medium';
  fullWidth?: boolean;
  label?: React.ReactNode;
  variant?: 'standard' | 'outlined' | 'filled';
  displayEmpty?: boolean;
  multiple?: boolean;
  renderValue?: (value: unknown) => React.ReactNode;
  onChange?: (event: { target: { value: string | number | string[] } }, child?: React.ReactNode) => void;
  sx?: SxObject;
}

export function Select({ size = 'medium', fullWidth, className, children, value, onChange, name, disabled, required, multiple, sx, style, ...rest }: SelectProps) {
  const sizeClass = size === 'small' ? 'px-2.5 py-1 text-sm' : 'px-3 py-2 text-sm';
  return (
    <select
      className={cn(inputBase, sizeClass, fullWidth && 'w-full', 'cursor-pointer', className)}
      value={value as string | number | readonly string[] | undefined}
      onChange={(e) => onChange?.({ target: { value: multiple ? Array.from(e.target.selectedOptions).map(o => o.value) : e.target.value } }) }
      name={name}
      disabled={disabled}
      required={required}
      multiple={multiple}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </select>
  );
}

interface MenuItemProps extends React.OptionHTMLAttributes<HTMLOptionElement> {
  value?: string | number;
  divider?: boolean;
  dense?: boolean;
  sx?: SxObject;
}

export function MenuItem({ value, children, sx, style, divider: _divider, dense: _dense, ...rest }: MenuItemProps) {
  return (
    <option value={value} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </option>
  );
}

interface FormControlLabelProps {
  control: React.ReactElement;
  label: React.ReactNode;
  labelPlacement?: 'end' | 'start' | 'top' | 'bottom';
  className?: string;
  disabled?: boolean;
  sx?: SxObject;
  value?: unknown;
}

export function FormControlLabel({ control, label, labelPlacement = 'end', className, disabled, sx }: FormControlLabelProps) {
  const dirCls = labelPlacement === 'start' ? 'flex-row-reverse' : labelPlacement === 'top' ? 'flex-col-reverse' : labelPlacement === 'bottom' ? 'flex-col' : 'flex-row';
  return (
    <label className={cn('inline-flex items-center gap-2', dirCls, disabled && 'opacity-50 cursor-not-allowed', className)} style={sxToStyle(sx)}>
      {control}
      <span className="text-sm text-foreground select-none">{label}</span>
    </label>
  );
}

interface CheckboxProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size' | 'color' | 'onChange'> {
  color?: string;
  size?: 'small' | 'medium';
  onChange?: (event: React.ChangeEvent<HTMLInputElement>, checked: boolean) => void;
}

export function Checkbox({ className, onChange, size = 'medium', color: _color, ...rest }: CheckboxProps) {
  const sz = size === 'small' ? 'h-3.5 w-3.5' : 'h-4 w-4';
  return (
    <input
      type="checkbox"
      className={cn(sz, 'rounded border-input text-primary focus:ring-ring', className)}
      onChange={(e) => onChange?.(e, e.target.checked)}
      {...rest}
    />
  );
}

interface SwitchProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size' | 'color' | 'onChange'> {
  color?: string;
  size?: 'small' | 'medium';
  onChange?: (event: React.ChangeEvent<HTMLInputElement>, checked: boolean) => void;
}

export function Switch({ className, onChange, checked, disabled, size: _size, color: _color, ...rest }: SwitchProps) {
  return (
    <label className={cn('relative inline-flex h-6 w-11 cursor-pointer items-center rounded-full transition-colors', checked ? 'bg-primary' : 'bg-muted', disabled && 'opacity-50 cursor-not-allowed', className)}>
      <input
        type="checkbox"
        className="sr-only"
        checked={checked}
        onChange={(e) => onChange?.(e, e.target.checked)}
        disabled={disabled}
        {...rest}
      />
      <span className={cn('inline-block h-4 w-4 transform rounded-full bg-background transition-transform', checked ? 'translate-x-6' : 'translate-x-1')} />
    </label>
  );
}

interface RadioProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size' | 'color'> {
  color?: string;
  size?: 'small' | 'medium';
}

export function Radio({ className, size = 'medium', color: _color, ...rest }: RadioProps) {
  const sz = size === 'small' ? 'h-3.5 w-3.5' : 'h-4 w-4';
  return (
    <input
      type="radio"
      className={cn(sz, 'border-input text-primary focus:ring-ring', className)}
      {...rest}
    />
  );
}

interface RadioGroupProps extends BoxProps {
  value?: string;
  onChange?: (event: React.ChangeEvent<HTMLInputElement>, value: string) => void;
  row?: boolean;
  name?: string;
}

export function RadioGroup({ value, onChange, row, name, className, children, sx, style, ...rest }: RadioGroupProps) {
  return (
    <div
      className={cn('flex', row ? 'flex-row gap-4' : 'flex-col gap-2', className)}
      style={{ ...sxToStyle(sx), ...style }}
      onChange={(e: React.ChangeEvent<HTMLInputElement>) => {
        if (e.target.tagName === 'INPUT') onChange?.(e, e.target.value);
      }}
      {...(rest as any)}
    >
      {React.Children.map(children, (child) => {
        if (React.isValidElement(child) && child.type === FormControlLabel) {
          const props = child.props as any;
          const control = props.control as any;
          if (control && React.isValidElement(control)) {
            const controlProps = control.props as any;
            return React.cloneElement(child as any, {
              control: React.cloneElement(control as any, {
                checked: value === props.value,
                value: props.value,
                name,
              }),
            });
          }
        }
        return child;
      })}
    </div>
  );
}

interface InputAdornmentProps extends BoxProps {
  position?: 'start' | 'end';
}

export function InputAdornment({ className, children, sx, style, ...rest }: InputAdornmentProps) {
  return (
    <span className={cn('inline-flex items-center text-muted-foreground', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </span>
  );
}

// -----------------------------------------------------------------------------
// Alert
// -----------------------------------------------------------------------------

interface AlertProps extends BoxProps {
  severity?: 'success' | 'warning' | 'info' | 'error';
  variant?: 'standard' | 'filled' | 'outlined';
  icon?: React.ReactNode | false;
  action?: React.ReactNode;
  onClose?: () => void;
}

export function Alert({ severity = 'info', variant = 'standard', icon, action, onClose, className, children, sx, style, ...rest }: AlertProps) {
  const colors: Record<string, string> = {
    success: 'bg-success-50 text-success-700 border-success-200 dark:bg-success-900/20 dark:text-success-300 dark:border-success-800',
    warning: 'bg-warning-50 text-warning-700 border-warning-200 dark:bg-warning-900/20 dark:text-warning-300 dark:border-warning-800',
    info: 'bg-info-50 text-info-700 border-info-200 dark:bg-info-900/20 dark:text-info-300 dark:border-info-800',
    error: 'bg-danger-50 text-danger-700 border-danger-200 dark:bg-danger-900/20 dark:text-danger-300 dark:border-danger-800',
  };
  return (
    <div
      role="alert"
      className={cn('flex items-start gap-3 rounded-md border p-3', colors[severity], className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {icon !== false && <span className="mt-0.5 flex-shrink-0">{icon}</span>}
      <div className="flex-1 min-w-0 text-sm">{children}</div>
      {action && <div className="flex-shrink-0">{action}</div>}
      {onClose && (
        <button type="button" onClick={onClose} className="flex-shrink-0 opacity-70 hover:opacity-100">
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}

export function AlertTitle({ className, children }: { className?: string; children?: React.ReactNode }) {
  return <div className={cn('font-semibold mb-0.5', className)}>{children}</div>;
}

// -----------------------------------------------------------------------------
// Chip / Badge / Avatar / Tooltip
// -----------------------------------------------------------------------------

interface ChipProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onClick'> {
  label?: React.ReactNode;
  variant?: 'filled' | 'outlined';
  color?: 'default' | 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success';
  size?: 'small' | 'medium';
  icon?: React.ReactNode;
  avatar?: React.ReactNode;
  deleteIcon?: React.ReactNode;
  onDelete?: () => void;
  onClick?: React.MouseEventHandler<HTMLDivElement>;
  clickable?: boolean;
  disabled?: boolean;
  sx?: SxObject;
}

export function Chip({ label, variant = 'filled', color = 'default', size = 'medium', icon, avatar, deleteIcon, onDelete, onClick, className, children, sx, style, ...rest }: ChipProps) {
  const filled: Record<string, string> = {
    default: 'bg-muted text-foreground',
    primary: 'bg-primary text-primary-foreground',
    secondary: 'bg-secondary text-secondary-foreground',
    error: 'bg-destructive text-destructive-foreground',
    warning: 'bg-warning text-text-inverse',
    info: 'bg-info text-text-inverse',
    success: 'bg-success text-text-inverse',
  };
  const outlined: Record<string, string> = {
    default: 'border border-border text-foreground',
    primary: 'border border-primary text-primary',
    secondary: 'border border-secondary text-secondary',
    error: 'border border-destructive text-destructive',
    warning: 'border border-warning text-warning',
    info: 'border border-info text-info',
    success: 'border border-success text-success',
  };
  const variants = variant === 'outlined' ? outlined : filled;
  const sizeCls = size === 'small' ? 'px-2 py-0.5 text-xs h-6' : 'px-2.5 py-1 text-sm h-7';
  return (
    <div
      role={onClick ? 'button' : undefined}
      onClick={onClick}
      className={cn(
        'inline-flex items-center gap-1 rounded-full font-medium transition-colors',
        variants[color],
        sizeCls,
        onClick && 'cursor-pointer hover:opacity-90',
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {avatar}
      {icon}
      <span className="truncate">{label ?? children}</span>
      {onDelete && (
        <button type="button" onClick={(e) => { e.stopPropagation(); onDelete(); }} className="opacity-70 hover:opacity-100">
          {deleteIcon ?? <X className="h-3 w-3" />}
        </button>
      )}
    </div>
  );
}

interface BadgeProps extends Omit<React.HTMLAttributes<HTMLSpanElement>, 'color'> {
  badgeContent?: React.ReactNode;
  color?: 'default' | 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success';
  variant?: 'standard' | 'dot';
  invisible?: boolean;
  max?: number;
  showZero?: boolean;
  overlap?: 'rectangular' | 'circular';
  anchorOrigin?: { vertical: 'top' | 'bottom'; horizontal: 'left' | 'right' };
  sx?: SxObject;
}

export function Badge({ badgeContent, color = 'default', variant = 'standard', invisible, max = 99, showZero, anchorOrigin, className, children, sx, style, ...rest }: BadgeProps) {
  const colors: Record<string, string> = {
    default: 'bg-muted text-foreground',
    primary: 'bg-primary text-primary-foreground',
    secondary: 'bg-secondary text-secondary-foreground',
    error: 'bg-destructive text-destructive-foreground',
    warning: 'bg-warning text-text-inverse',
    info: 'bg-info text-text-inverse',
    success: 'bg-success text-text-inverse',
  };
  const v = anchorOrigin?.vertical ?? 'top';
  const h = anchorOrigin?.horizontal ?? 'right';
  const pos = `${v === 'top' ? '-top-1' : '-bottom-1'} ${h === 'right' ? '-right-1' : '-left-1'}`;
  const showContent = !invisible && (showZero || badgeContent !== 0) && badgeContent !== undefined && badgeContent !== null;
  const displayContent = typeof badgeContent === 'number' && badgeContent > max ? `${max}+` : badgeContent;
  return (
    <span className={cn('relative inline-flex', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
      {showContent && (
        variant === 'dot'
          ? <span className={cn('absolute h-2 w-2 rounded-full ring-2 ring-background', colors[color], pos)} />
          : <span className={cn('absolute inline-flex items-center justify-center min-w-[18px] h-[18px] px-1 rounded-full text-[10px] font-medium ring-2 ring-background', colors[color], pos)}>{displayContent}</span>
      )}
    </span>
  );
}

interface AvatarProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'children'> {
  src?: string;
  alt?: string;
  variant?: 'circular' | 'rounded' | 'square';
  sizes?: string;
  imgProps?: React.ImgHTMLAttributes<HTMLImageElement>;
  children?: React.ReactNode;
  sx?: SxObject;
}

export function Avatar({ src, alt, variant = 'circular', className, children, sx, style, imgProps, ...rest }: AvatarProps) {
  const shape = variant === 'square' ? 'rounded-none' : variant === 'rounded' ? 'rounded-md' : 'rounded-full';
  return (
    <div
      className={cn('inline-flex items-center justify-center bg-muted text-muted-foreground overflow-hidden h-10 w-10 text-sm font-medium', shape, className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {src ? <img src={src} alt={alt} className="h-full w-full object-cover" {...imgProps} /> : children}
    </div>
  );
}

interface TooltipProps {
  title: React.ReactNode;
  children: React.ReactElement;
  placement?: 'top' | 'bottom' | 'left' | 'right';
  arrow?: boolean;
  open?: boolean;
  enterDelay?: number;
}

export function Tooltip({ title, children, placement = 'top' }: TooltipProps) {
  const [open, setOpen] = React.useState(false);
  const ref = React.useRef<HTMLSpanElement>(null);
  const pos = {
    top: 'bottom-full left-1/2 -translate-x-1/2 mb-1',
    bottom: 'top-full left-1/2 -translate-x-1/2 mt-1',
    left: 'right-full top-1/2 -translate-y-1/2 mr-1',
    right: 'left-full top-1/2 -translate-y-1/2 ml-1',
  }[placement];

  // Wrap children in a span (without modifying their props) so we can attach
  // hover handlers without breaking refs / focus on the original element.
  return (
    <span
      ref={ref}
      className="relative inline-flex"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
      onFocus={() => setOpen(true)}
      onBlur={() => setOpen(false)}
    >
      {children}
      {open && title && (
        <span className={cn('absolute z-50 px-2 py-1 rounded bg-popover text-popover-foreground border border-border text-xs shadow-md whitespace-nowrap', pos)} role="tooltip">
          {title}
        </span>
      )}
    </span>
  );
}

// -----------------------------------------------------------------------------
// List / ListItem / ListItemText / ListItemIcon / ListItemSecondaryAction / ListSubheader
// -----------------------------------------------------------------------------

interface ListProps extends BoxProps {
  dense?: boolean;
  disablePadding?: boolean;
  subheader?: React.ReactNode;
  component?: React.ElementType;
}

export function List({ children, className, dense, disablePadding, subheader, sx, style, component, ...rest }: ListProps) {
  const Comp = (component ?? 'ul') as React.ElementType;
  return (
    <Comp className={cn('list-none', !disablePadding && 'py-2', dense && 'text-sm', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {subheader}
      {children}
    </Comp>
  );
}

interface ListItemProps extends Omit<React.LiHTMLAttributes<HTMLLIElement>, 'color'> {
  button?: boolean;
  divider?: boolean;
  disabled?: boolean;
  selected?: boolean;
  disablePadding?: boolean;
  alignItems?: 'flex-start' | 'center';
  secondaryAction?: React.ReactNode;
  component?: React.ElementType;
  sx?: SxObject;
}

export function ListItem({ button, divider, selected, disablePadding, alignItems, secondaryAction, component, className, children, sx, style, ...rest }: ListItemProps) {
  const Comp = (component ?? 'li') as React.ElementType;
  return (
    <Comp
      className={cn(
        'flex items-center gap-3',
        !disablePadding && 'px-4 py-2',
        alignItems === 'flex-start' && 'items-start',
        button && 'cursor-pointer hover:bg-accent hover:text-accent-foreground',
        selected && 'bg-accent text-accent-foreground',
        divider && 'border-b border-border',
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
      {secondaryAction && <div className="ml-auto">{secondaryAction}</div>}
    </Comp>
  );
}

export function ListItemButton({ className, children, selected, sx, style, ...rest }: BoxProps & { selected?: boolean }) {
  return (
    <button
      type="button"
      className={cn('w-full flex items-center gap-3 px-4 py-2 text-left hover:bg-accent hover:text-accent-foreground', selected && 'bg-accent text-accent-foreground', className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...(rest as any)}
    >
      {children}
    </button>
  );
}

interface ListItemTextProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  primary?: React.ReactNode;
  secondary?: React.ReactNode;
  primaryTypographyProps?: { className?: string };
  secondaryTypographyProps?: { className?: string };
  disableTypography?: boolean;
  inset?: boolean;
  sx?: SxObject;
}

export function ListItemText({ primary, secondary, primaryTypographyProps, secondaryTypographyProps, inset, className, children, sx, style, ...rest }: ListItemTextProps) {
  return (
    <div className={cn('flex-1 min-w-0', inset && 'pl-12', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {primary !== undefined && <div className={cn('text-sm text-foreground', primaryTypographyProps?.className)}>{primary}</div>}
      {secondary !== undefined && <div className={cn('text-xs text-muted-foreground', secondaryTypographyProps?.className)}>{secondary}</div>}
      {children}
    </div>
  );
}

export function ListItemIcon({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <span className={cn('inline-flex items-center justify-center text-muted-foreground min-w-[28px]', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </span>
  );
}

export function ListItemSecondaryAction({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <span className={cn('ml-auto inline-flex items-center', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </span>
  );
}

export function ListSubheader({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('px-4 py-1 text-xs uppercase tracking-wider text-muted-foreground', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Table family
// -----------------------------------------------------------------------------

interface TableProps extends Omit<React.TableHTMLAttributes<HTMLTableElement>, 'color'> {
  size?: 'small' | 'medium';
  stickyHeader?: boolean;
  sx?: SxObject;
}

export function Table({ size, stickyHeader, className, children, sx, style, ...rest }: TableProps) {
  return (
    <table
      className={cn('w-full text-left text-sm border-collapse', className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </table>
  );
}

export function TableHead({ className, children, sx, style, ...rest }: React.HTMLAttributes<HTMLTableSectionElement> & { sx?: SxObject }) {
  return (
    <thead className={cn('bg-muted text-muted-foreground', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </thead>
  );
}

export function TableBody({ className, children, sx, style, ...rest }: React.HTMLAttributes<HTMLTableSectionElement> & { sx?: SxObject }) {
  return (
    <tbody className={cn('divide-y divide-border', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </tbody>
  );
}

interface TableRowProps extends Omit<React.HTMLAttributes<HTMLTableRowElement>, 'color'> {
  hover?: boolean;
  selected?: boolean;
  sx?: SxObject;
}

export function TableRow({ hover, selected, className, children, sx, style, ...rest }: TableRowProps) {
  return (
    <tr
      className={cn(
        hover && 'hover:bg-accent hover:text-accent-foreground',
        selected && 'bg-accent text-accent-foreground',
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </tr>
  );
}

interface TableCellProps extends Omit<React.TdHTMLAttributes<HTMLTableCellElement>, 'color' | 'align'> {
  component?: React.ElementType;
  align?: 'left' | 'center' | 'right' | 'justify' | 'inherit';
  padding?: 'normal' | 'checkbox' | 'none';
  scope?: string;
  sortDirection?: 'asc' | 'desc' | false;
  sx?: SxObject;
}

export function TableCell({ component, align, padding, className, children, sx, style, ...rest }: TableCellProps) {
  const Comp = (component ?? 'td') as React.ElementType;
  return (
    <Comp
      className={cn(
        padding === 'checkbox' ? 'px-2 py-1' : padding === 'none' ? '' : 'px-4 py-2',
        align && `text-${align}`,
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </Comp>
  );
}

interface TableContainerProps extends BoxProps {
  component?: React.ElementType;
}

export function TableContainer({ component, className, children, sx, style, ...rest }: TableContainerProps) {
  const Comp = (component ?? 'div') as React.ElementType;
  return (
    <Comp className={cn('w-full overflow-x-auto', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </Comp>
  );
}

export function TablePagination(props: any) {
  const { count, page = 0, rowsPerPage = 10, onPageChange, rowsPerPageOptions, onRowsPerPageChange, sx, ...rest } = props;
  return (
    <div className="flex items-center justify-end gap-3 px-4 py-2 text-sm text-muted-foreground" style={sxToStyle(sx)} {...rest}>
      <span>{page * rowsPerPage + 1}-{Math.min((page + 1) * rowsPerPage, count)} of {count}</span>
      <button type="button" disabled={page === 0} onClick={(e) => onPageChange?.(e, page - 1)} className="px-2 disabled:opacity-50">‹</button>
      <button type="button" disabled={(page + 1) * rowsPerPage >= count} onClick={(e) => onPageChange?.(e, page + 1)} className="px-2 disabled:opacity-50">›</button>
      {rowsPerPageOptions && onRowsPerPageChange && (
        <select className={cn(inputBase, 'px-2 py-0.5 text-xs')} value={rowsPerPage} onChange={onRowsPerPageChange}>
          {(rowsPerPageOptions as number[]).map((n) => <option key={n} value={n}>{n} / page</option>)}
        </select>
      )}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Tabs / Tab
// -----------------------------------------------------------------------------

interface TabsProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onChange'> {
  value: string | number | false;
  onChange?: (event: React.SyntheticEvent, value: any) => void;
  variant?: 'standard' | 'fullWidth' | 'scrollable';
  orientation?: 'horizontal' | 'vertical';
  centered?: boolean;
  textColor?: 'primary' | 'secondary' | 'inherit';
  indicatorColor?: 'primary' | 'secondary';
  sx?: SxObject;
  TabIndicatorProps?: { sx?: SxObject; className?: string };
}

const TabsContext = React.createContext<{ value: any; onChange?: (e: any, v: any) => void }>({ value: undefined });

export function Tabs({ value, onChange, variant, orientation = 'horizontal', centered, className, children, sx, style, ...rest }: TabsProps) {
  return (
    <TabsContext.Provider value={{ value, onChange }}>
      <div
        className={cn(
          'flex',
          orientation === 'vertical' ? 'flex-col border-r border-border' : 'flex-row border-b border-border',
          centered && 'justify-center',
          variant === 'fullWidth' && '[&>*]:flex-1',
          variant === 'scrollable' && 'overflow-x-auto',
          className,
        )}
        style={{ ...sxToStyle(sx), ...style }}
        {...rest}
      >
        {children}
      </div>
    </TabsContext.Provider>
  );
}

interface TabProps extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'color' | 'onChange' | 'value'> {
  label?: React.ReactNode;
  value?: string | number;
  icon?: React.ReactNode;
  iconPosition?: 'start' | 'end' | 'top' | 'bottom';
  disabled?: boolean;
  wrapped?: boolean;
  sx?: SxObject;
}

export function Tab({ label, value, icon, iconPosition = 'top', disabled, className, children, sx, style, ...rest }: TabProps) {
  const ctx = React.useContext(TabsContext);
  const selected = ctx.value === value;
  const dir = iconPosition === 'top' || iconPosition === 'bottom' ? 'flex-col' : 'flex-row';
  return (
    <button
      type="button"
      role="tab"
      aria-selected={selected}
      disabled={disabled}
      onClick={(e) => ctx.onChange?.(e, value)}
      className={cn(
        'inline-flex items-center justify-center gap-2 px-4 py-2 text-sm font-medium transition-colors border-b-2',
        dir,
        selected ? 'text-primary border-primary' : 'text-muted-foreground border-transparent hover:text-foreground hover:border-border',
        disabled && 'opacity-50 cursor-not-allowed',
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {icon}
      {label ?? children}
    </button>
  );
}

// -----------------------------------------------------------------------------
// Stepper / Step / StepLabel / StepContent
// -----------------------------------------------------------------------------

interface StepperProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  activeStep?: number;
  alternativeLabel?: boolean;
  orientation?: 'horizontal' | 'vertical';
  sx?: SxObject;
}

const StepperContext = React.createContext<{ activeStep: number; orientation: 'horizontal' | 'vertical' }>({ activeStep: 0, orientation: 'horizontal' });

export function Stepper({ activeStep = 0, orientation = 'horizontal', alternativeLabel, className, children, sx, style, ...rest }: StepperProps) {
  return (
    <StepperContext.Provider value={{ activeStep, orientation }}>
      <div
        className={cn(orientation === 'vertical' ? 'flex flex-col gap-3' : 'flex items-center gap-3 overflow-x-auto', className)}
        style={{ ...sxToStyle(sx), ...style }}
        {...rest}
      >
        {React.Children.map(children, (child, idx) => {
          if (!React.isValidElement(child)) return child;
          return React.cloneElement(child as React.ReactElement<any>, { index: idx, total: React.Children.count(children) });
        })}
      </div>
    </StepperContext.Provider>
  );
}

interface StepProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  active?: boolean;
  completed?: boolean;
  disabled?: boolean;
  index?: number;
  total?: number;
  sx?: SxObject;
}

const StepContext = React.createContext<{ index: number; active: boolean; completed: boolean; isLast: boolean }>({ index: 0, active: false, completed: false, isLast: false });

export function Step({ active, completed, disabled, index = 0, total = 0, className, children, sx, style, ...rest }: StepProps) {
  const stepper = React.useContext(StepperContext);
  const isActive = active ?? stepper.activeStep === index;
  const isCompleted = completed ?? stepper.activeStep > index;
  const isLast = index === total - 1;
  return (
    <StepContext.Provider value={{ index, active: isActive, completed: isCompleted, isLast }}>
      <div
        className={cn(
          'flex items-start gap-2 flex-shrink-0',
          stepper.orientation === 'vertical' ? 'flex-col' : '',
          disabled && 'opacity-50',
          className,
        )}
        style={{ ...sxToStyle(sx), ...style }}
        {...rest}
      >
        {children}
      </div>
    </StepContext.Provider>
  );
}

interface StepLabelProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  optional?: React.ReactNode;
  icon?: React.ReactNode;
  StepIconComponent?: React.ComponentType<{ active?: boolean; completed?: boolean }>;
  sx?: SxObject;
}

export function StepLabel({ optional, icon, className, children, sx, style, ...rest }: StepLabelProps) {
  const step = React.useContext(StepContext);
  return (
    <div className={cn('flex items-center gap-2', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      <span className={cn(
        'inline-flex items-center justify-center h-6 w-6 rounded-full text-xs font-medium',
        step.completed ? 'bg-primary text-primary-foreground' : step.active ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground',
      )}>
        {icon ?? step.index + 1}
      </span>
      <span className={cn('text-sm', step.active ? 'font-medium text-foreground' : 'text-muted-foreground')}>{children}</span>
      {optional && <span className="text-xs text-muted-foreground">{optional}</span>}
    </div>
  );
}

export function StepContent({ className, children, sx, style, ...rest }: BoxProps) {
  const step = React.useContext(StepContext);
  if (!step.active) return null;
  return (
    <div className={cn('mt-2 ml-8', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Accordion family
// -----------------------------------------------------------------------------

interface AccordionProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onChange'> {
  expanded?: boolean;
  defaultExpanded?: boolean;
  onChange?: (event: React.SyntheticEvent, expanded: boolean) => void;
  disabled?: boolean;
  square?: boolean;
  sx?: SxObject;
}

const AccordionContext = React.createContext<{ expanded: boolean; toggle: (e?: React.SyntheticEvent) => void }>({ expanded: false, toggle: () => undefined });

export function Accordion({ expanded: controlled, defaultExpanded, onChange, disabled, square, className, children, sx, style, ...rest }: AccordionProps) {
  const [internal, setInternal] = React.useState(!!defaultExpanded);
  const expanded = controlled ?? internal;
  const toggle = (e?: React.SyntheticEvent) => {
    if (disabled) return;
    const next = !expanded;
    if (controlled === undefined) setInternal(next);
    onChange?.(e ?? ({} as React.SyntheticEvent), next);
  };
  return (
    <AccordionContext.Provider value={{ expanded, toggle }}>
      <div
        className={cn('border border-border bg-card text-card-foreground', !square && 'rounded-md', className)}
        style={{ ...sxToStyle(sx), ...style }}
        {...rest}
      >
        {children}
      </div>
    </AccordionContext.Provider>
  );
}

interface AccordionSummaryProps extends BoxProps {
  expandIcon?: React.ReactNode;
}

export function AccordionSummary({ expandIcon, className, children, sx, style, ...rest }: AccordionSummaryProps) {
  const ctx = React.useContext(AccordionContext);
  return (
    <button
      type="button"
      onClick={() => ctx.toggle()}
      className={cn('flex w-full items-center justify-between px-4 py-3 text-left text-sm font-medium hover:bg-accent hover:text-accent-foreground', className)}
      style={{ ...sxToStyle(sx), ...style }}
      {...(rest as any)}
    >
      <span className="flex-1 min-w-0">{children}</span>
      <span className={cn('transition-transform', ctx.expanded && 'rotate-180')}>{expandIcon ?? <ChevronDown className="h-4 w-4" />}</span>
    </button>
  );
}

export function AccordionDetails({ className, children, sx, style, ...rest }: BoxProps) {
  const ctx = React.useContext(AccordionContext);
  if (!ctx.expanded) return null;
  return (
    <div className={cn('px-4 py-3 border-t border-border', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function AccordionActions({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('flex items-center justify-end gap-2 px-4 py-2 border-t border-border', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Progress
// -----------------------------------------------------------------------------

interface LinearProgressProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  variant?: 'determinate' | 'indeterminate' | 'buffer' | 'query';
  value?: number;
  valueBuffer?: number;
  color?: 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success' | 'inherit';
  sx?: SxObject;
}

export function LinearProgress({ variant = 'indeterminate', value = 0, color = 'primary', className, sx, style, ...rest }: LinearProgressProps) {
  const colors: Record<string, string> = {
    primary: 'bg-primary',
    secondary: 'bg-secondary',
    error: 'bg-destructive',
    warning: 'bg-warning',
    info: 'bg-info',
    success: 'bg-success',
    inherit: 'bg-current',
  };
  return (
    <div className={cn('relative w-full h-1 overflow-hidden rounded-full bg-muted', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {variant === 'determinate' ? (
        <div className={cn('h-full transition-all', colors[color])} style={{ width: `${Math.min(100, Math.max(0, value))}%` }} />
      ) : (
        <div className={cn('h-full w-1/3 absolute left-0 animate-[loading-shimmer_1.5s_linear_infinite]', colors[color])} />
      )}
    </div>
  );
}

interface CircularProgressProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  variant?: 'determinate' | 'indeterminate';
  value?: number;
  size?: number;
  thickness?: number;
  color?: 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success' | 'inherit';
  sx?: SxObject;
}

export function CircularProgress({ variant = 'indeterminate', value = 0, size = 40, thickness = 3.6, color = 'primary', className, sx, style, ...rest }: CircularProgressProps) {
  const colors: Record<string, string> = {
    primary: 'text-primary',
    secondary: 'text-secondary',
    error: 'text-destructive',
    warning: 'text-warning',
    info: 'text-info',
    success: 'text-success',
    inherit: 'text-current',
  };
  const radius = (size - thickness) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (value / 100) * circumference;
  return (
    <div
      className={cn('inline-block', variant === 'indeterminate' && 'animate-spin', className)}
      style={{ width: size, height: size, ...sxToStyle(sx), ...style }}
      {...rest}
    >
      <svg viewBox={`0 0 ${size} ${size}`} className={colors[color]}>
        <circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke="currentColor" strokeWidth={thickness} opacity={0.2} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="none"
          stroke="currentColor"
          strokeWidth={thickness}
          strokeDasharray={circumference}
          strokeDashoffset={variant === 'determinate' ? offset : circumference * 0.75}
          strokeLinecap="round"
          transform={`rotate(-90 ${size / 2} ${size / 2})`}
        />
      </svg>
    </div>
  );
}

// -----------------------------------------------------------------------------
// Snackbar
// -----------------------------------------------------------------------------

interface SnackbarProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onClose'> {
  open: boolean;
  message?: React.ReactNode;
  autoHideDuration?: number;
  onClose?: (event?: React.SyntheticEvent | Event, reason?: string) => void;
  anchorOrigin?: { vertical: 'top' | 'bottom'; horizontal: 'left' | 'center' | 'right' };
  action?: React.ReactNode;
  sx?: SxObject;
}

export function Snackbar({ open, message, autoHideDuration, onClose, anchorOrigin, action, className, children, sx, style, ...rest }: SnackbarProps) {
  React.useEffect(() => {
    if (!open || !autoHideDuration) return;
    const id = window.setTimeout(() => onClose?.(undefined, 'timeout'), autoHideDuration);
    return () => window.clearTimeout(id);
  }, [open, autoHideDuration, onClose]);
  if (!open) return null;
  const v = anchorOrigin?.vertical ?? 'bottom';
  const h = anchorOrigin?.horizontal ?? 'left';
  const pos = cn(
    v === 'top' ? 'top-4' : 'bottom-4',
    h === 'left' ? 'left-4' : h === 'right' ? 'right-4' : 'left-1/2 -translate-x-1/2',
  );
  return (
    <div className={cn('fixed z-[60]', pos, className)} style={{ ...sxToStyle(sx), ...style }} role="alert" {...rest}>
      {children ?? (
        <div className="flex items-center gap-2 rounded-md bg-popover text-popover-foreground border border-border px-4 py-2 shadow-lg">
          <span className="text-sm">{message}</span>
          {action}
        </div>
      )}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Rating
// -----------------------------------------------------------------------------

interface RatingProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color' | 'onChange'> {
  value?: number | null;
  defaultValue?: number;
  precision?: number;
  max?: number;
  readOnly?: boolean;
  size?: 'small' | 'medium' | 'large';
  onChange?: (event: React.SyntheticEvent, value: number | null) => void;
  emptyIcon?: React.ReactNode;
  icon?: React.ReactNode;
  sx?: SxObject;
}

export function Rating({ value, defaultValue = 0, max = 5, readOnly, size = 'medium', onChange, className, sx, style, ...rest }: RatingProps) {
  const [internal, setInternal] = React.useState(defaultValue);
  const v = value ?? internal;
  const sz = size === 'small' ? 'h-3.5 w-3.5' : size === 'large' ? 'h-6 w-6' : 'h-4 w-4';
  return (
    <div className={cn('inline-flex items-center gap-0.5', className)} style={{ ...sxToStyle(sx), ...style }} role="radiogroup" {...rest}>
      {Array.from({ length: max }).map((_, i) => {
        const on = i < (v ?? 0);
        return (
          <button
            type="button"
            key={i}
            disabled={readOnly}
            onClick={(e) => {
              if (readOnly) return;
              const next = i + 1;
              setInternal(next);
              onChange?.(e, next);
            }}
            className={cn('p-0.5', readOnly ? 'cursor-default' : 'cursor-pointer')}
            aria-label={`${i + 1} stars`}
          >
            <svg viewBox="0 0 24 24" className={cn(sz, on ? 'fill-warning text-warning' : 'fill-muted text-muted')}>
              <path d="M12 .587l3.668 7.431 8.2 1.192-5.934 5.787 1.401 8.165L12 18.897l-7.335 3.864L6.066 14.6.132 8.81l8.2-1.192z" />
            </svg>
          </button>
        );
      })}
    </div>
  );
}

// -----------------------------------------------------------------------------
// Skeleton, Backdrop, ButtonGroup, Toolbar, AppBar — small utilities
// -----------------------------------------------------------------------------

interface SkeletonProps extends Omit<React.HTMLAttributes<HTMLDivElement>, 'color'> {
  variant?: 'text' | 'rectangular' | 'circular' | 'rounded';
  width?: number | string;
  height?: number | string;
  animation?: 'pulse' | 'wave' | false;
  sx?: SxObject;
}

export function Skeleton({ variant = 'text', width, height, animation = 'pulse', className, sx, style, ...rest }: SkeletonProps) {
  const shapes = { text: 'rounded h-4', rectangular: '', circular: 'rounded-full', rounded: 'rounded-md' };
  return (
    <div
      className={cn('bg-muted', animation === 'pulse' && 'animate-pulse', shapes[variant], className)}
      style={{ width, height: height ?? (variant === 'text' ? undefined : 32), ...sxToStyle(sx), ...style }}
      {...rest}
    />
  );
}

export function Backdrop({ open, onClick, className, children, sx, style, ...rest }: BoxProps & { open?: boolean; onClick?: React.MouseEventHandler<HTMLDivElement> }) {
  if (!open) return null;
  return (
    <div className={cn('fixed inset-0 z-40 bg-bg-overlay flex items-center justify-center', className)} style={{ ...sxToStyle(sx), ...style }} onClick={onClick} {...rest}>
      {children}
    </div>
  );
}

export function ButtonGroup({ className, children, variant: _variant, color: _color, size: _size, orientation, sx, style, ...rest }: BoxProps & { variant?: string; color?: string; size?: string; orientation?: 'horizontal' | 'vertical' }) {
  return (
    <div className={cn('inline-flex', orientation === 'vertical' ? 'flex-col' : 'flex-row', '[&>*:not(:first-child)]:rounded-l-none [&>*:not(:last-child)]:rounded-r-none [&>*:not(:last-child)]:border-r-0', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function Toolbar({ className, children, sx, style, ...rest }: BoxProps) {
  return (
    <div className={cn('flex items-center min-h-[56px] px-4', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function AppBar({ className, children, position, color: _color, sx, style, ...rest }: BoxProps & { position?: 'static' | 'fixed' | 'absolute' | 'sticky' | 'relative'; color?: string }) {
  return (
    <header
      className={cn(
        'bg-card text-card-foreground border-b border-border shadow-sm',
        position === 'fixed' && 'fixed top-0 left-0 right-0 z-40',
        position === 'sticky' && 'sticky top-0 z-30',
        className,
      )}
      style={{ ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </header>
  );
}

// -----------------------------------------------------------------------------
// Collapse / Fade — minimal animations (just show/hide)
// -----------------------------------------------------------------------------

export function Collapse({ in: open, children, className, sx, style, timeout: _timeout, orientation, ...rest }: BoxProps & { in?: boolean; timeout?: number; orientation?: 'horizontal' | 'vertical' }) {
  if (!open) return null;
  return (
    <div className={cn('overflow-hidden', orientation === 'horizontal' ? 'transition-[width] duration-200' : 'transition-[max-height] duration-200', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {children}
    </div>
  );
}

export function Fade({ in: open, children }: { in?: boolean; children: React.ReactElement; timeout?: number }) {
  if (!open) return null;
  return children;
}

export function Grow({ in: open, children }: { in?: boolean; children: React.ReactElement; timeout?: number }) {
  if (!open) return null;
  return children;
}

export function Slide({ in: open, children }: { in?: boolean; children: React.ReactElement; direction?: string; timeout?: number }) {
  if (!open) return null;
  return children;
}

export function Zoom({ in: open, children }: { in?: boolean; children: React.ReactElement; timeout?: number }) {
  if (!open) return null;
  return children;
}

// -----------------------------------------------------------------------------
// useMediaQuery / useTheme — minimal
// -----------------------------------------------------------------------------

export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = React.useState(false);
  React.useEffect(() => {
    if (typeof window === 'undefined') return;
    const mq = window.matchMedia(query);
    const onChange = () => setMatches(mq.matches);
    onChange();
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, [query]);
  return matches;
}

export function useTheme() {
  // Minimal stub — return an object compatible with common MUI theme reads.
  return {
    breakpoints: {
      up: (k: 'xs' | 'sm' | 'md' | 'lg' | 'xl') => `(min-width: ${{ xs: 0, sm: 600, md: 900, lg: 1200, xl: 1536 }[k]}px)`,
      down: (k: 'xs' | 'sm' | 'md' | 'lg' | 'xl') => `(max-width: ${{ xs: 599, sm: 899, md: 1199, lg: 1535, xl: 9999 }[k]}px)`,
    },
    palette: {
      mode: typeof document !== 'undefined' && document.documentElement.classList.contains('dark') ? 'dark' : 'light',
      primary: { main: 'hsl(var(--primary))' },
      secondary: { main: 'hsl(var(--secondary))' },
      error: { main: 'hsl(var(--destructive))' },
    },
    spacing: (n: number) => n * SPACING_UNIT,
  };
}

// -----------------------------------------------------------------------------
// Link / AvatarGroup
// -----------------------------------------------------------------------------

interface LinkProps extends Omit<React.AnchorHTMLAttributes<HTMLAnchorElement>, 'color'> {
  color?: 'primary' | 'secondary' | 'inherit' | string;
  underline?: 'none' | 'hover' | 'always';
  variant?: TypoVariant;
  component?: React.ElementType;
  sx?: SxObject;
}

export function Link({ color = 'primary', underline = 'hover', variant, component, className, children, sx, style, ...rest }: LinkProps) {
  const Comp = (component ?? 'a') as React.ElementType;
  const colorCls =
    color === 'primary' ? 'text-primary' :
    color === 'secondary' ? 'text-secondary' :
    color === 'inherit' ? 'text-inherit' : '';
  const underlineCls =
    underline === 'always' ? 'underline' :
    underline === 'none' ? 'no-underline' : 'hover:underline';
  return (
    <Comp
      className={cn(colorCls, underlineCls, variant && TYPO_CLASS[variant], className)}
      style={{ color: !colorCls ? colorVal(color) : undefined, ...sxToStyle(sx), ...style }}
      {...rest}
    >
      {children}
    </Comp>
  );
}

interface AvatarGroupProps extends BoxProps {
  max?: number;
  spacing?: 'small' | 'medium' | number;
  total?: number;
}

export function AvatarGroup({ max = 5, spacing: avatarSpacing = 'medium', total, className, children, sx, style, ...rest }: AvatarGroupProps) {
  const items = React.Children.toArray(children);
  const visible = items.slice(0, max);
  const overflow = (total ?? items.length) - visible.length;
  const overlap = avatarSpacing === 'small' ? '-ml-3' : typeof avatarSpacing === 'number' ? '' : '-ml-2';
  return (
    <div className={cn('inline-flex items-center', className)} style={{ ...sxToStyle(sx), ...style }} {...rest}>
      {visible.map((c, i) => (
        <span key={i} className={cn('inline-block ring-2 ring-background rounded-full', i > 0 && overlap)} style={typeof avatarSpacing === 'number' ? (i > 0 ? { marginLeft: -avatarSpacing } : undefined) : undefined}>
          {c}
        </span>
      ))}
      {overflow > 0 && (
        <span className={cn('inline-flex items-center justify-center h-10 w-10 rounded-full bg-muted text-muted-foreground text-xs font-medium ring-2 ring-background', overlap)}>+{overflow}</span>
      )}
    </div>
  );
}

// Re-exports as namespace too for compatibility
export const styled = (component: React.ElementType) => () => component;
export type Theme = ReturnType<typeof useTheme>;
