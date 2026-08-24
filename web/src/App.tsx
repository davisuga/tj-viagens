import { Navigate, Route, Routes } from 'react-router-dom';
import { Login } from '@/pages/Login';

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="*" element={<Navigate to="/login" replace />} />
    </Routes>
  );
}
