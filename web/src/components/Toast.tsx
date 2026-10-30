import React from 'react';
import { CheckCircle2 } from 'lucide-react';

interface ToastProps {
  message: string | null;
}

export const Toast: React.FC<ToastProps> = ({ message }) => {
  if (!message) return null;

  return (
    <div className="fixed bottom-6 right-6 z-50 flex items-center gap-2 bg-accent/90 text-accent border border-accent/40 px-4 py-3 rounded-lg shadow-sm backdrop-blur-md animate-bounce">
      <CheckCircle2 className="w-5 h-5 text-accent" />
      <span className="text-sm font-medium">{message}</span>
    </div>
  );
};
