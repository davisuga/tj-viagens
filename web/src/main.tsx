import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { Toaster } from 'sonner';
import { App } from './App';
import { AuthProvider } from './lib/auth';
import './index.css';

const queryClient = new QueryClient({
  // networkMode: 'always' — the default 'online' mode defers a failed query's
  // retry/error to the browser's onlineManager and can pause it indefinitely
  // (fetchStatus stuck 'paused', isError never true) if that manager's belief
  // about connectivity ever disagrees with reality. This is a same-machine/
  // intranet API with no offline-first requirement, so always attempt the
  // request and let retry/error settle on its own regardless of perceived
  // connectivity — what the isError gates below need to reliably fire.
  defaultOptions: { queries: { refetchOnWindowFocus: true, retry: 1, networkMode: 'always' } },
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthProvider>
          <App />
          <Toaster richColors position="top-center" />
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
