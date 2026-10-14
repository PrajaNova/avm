import { createContext, useCallback, useContext, useState } from 'react';
import { Toast } from './Toast';

const CopyContext = createContext<(text: string) => void>(() => {});

export const useCopy = () => useContext(CopyContext);

export const CopyProvider = ({ children }: { children: React.ReactNode }) => {
  const [message, setMessage] = useState<string | null>(null);
  const copy = useCallback((text: string) => {
    navigator.clipboard.writeText(text);
    setMessage(`Copied: ${text}`);
    setTimeout(() => setMessage(null), 2500);
  }, []);
  return (
    <CopyContext.Provider value={copy}>
      <Toast message={message} />
      {children}
    </CopyContext.Provider>
  );
};
