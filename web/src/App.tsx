import { Navigate, Route, Routes } from 'react-router-dom';
import { RequireRole } from '@/lib/auth';
import { Login } from '@/pages/Login';
import { Register } from '@/pages/Register';
import { SupplierHome } from '@/pages/supplier/Home';
import { SupplierQuotationPage } from '@/pages/supplier/QuotationBid';

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/registro" element={<Register />} />
      <Route
        path="/fornecedor"
        element={
          <RequireRole roles={['FORNECEDOR']}>
            <SupplierHome />
          </RequireRole>
        }
      />
      <Route
        path="/fornecedor/cotacoes/:id"
        element={
          <RequireRole roles={['FORNECEDOR']}>
            <SupplierQuotationPage />
          </RequireRole>
        }
      />
      <Route path="*" element={<Navigate to="/login" replace />} />
    </Routes>
  );
}
