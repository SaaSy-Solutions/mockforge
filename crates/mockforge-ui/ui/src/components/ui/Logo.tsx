import { useState } from 'react';

interface LogoProps {
  variant?: 'full' | 'icon';
  size?: 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
}

const sizeClasses = {
  sm: 'h-6 w-auto',
  md: 'h-8 w-auto',
  lg: 'h-10 w-auto',
  xl: 'h-12 w-auto'
};

export function Logo({ variant = 'full', size = 'md', className = '' }: LogoProps) {
  const [imageError, setImageError] = useState(false);

  // Select appropriate asset based on variant and size for optimal display
  const getLogoSrc = () => {
    if (variant === 'icon') {
      switch (size) {
        case 'md': return '/mockforge-icon-32.png'; // 32px for md size
        case 'xl': return '/mockforge-icon-48.png'; // 48px for xl size
        default: return '/mockforge-icon.png'; // fallback for other sizes
      }
    } else {
      switch (size) {
        case 'lg': return '/mockforge-logo-40.png'; // 40px height for lg size
        case 'xl': return '/mockforge-logo-80.png'; // 80px height for xl size
        default: return '/mockforge-logo.png'; // fallback for other sizes
      }
    }
  };

  const logoSrc = getLogoSrc();
  const altText = variant === 'icon' ? 'MockForge' : 'MockForge Logo';

  if (imageError) {
    return (
      <div className={`flex items-center ${className}`}>
        <div className={`bg-gradient-to-br from-orange-500 to-red-600 rounded-lg px-3 py-1 ${sizeClasses[size]} flex items-center justify-center text-white font-bold text-sm`}>
          {variant === 'icon' ? 'M' : 'MockForge'}
        </div>
      </div>
    );
  }

  return (
    <img
      src={logoSrc}
      alt={altText}
      className={`${sizeClasses[size]} ${className}`}
      onError={() => setImageError(true)}
    />
  );
}
