import { Navigate, Route, Routes } from 'react-router-dom';
import { RequireRole } from '@/lib/auth';
import { Login } from '@/pages/Login';
import { Register } from '@/pages/Register';
import { StaffDashboard } from '@/pages/staff/Dashboard';
import { NewQuotation } from '@/pages/staff/NewQuotation';
import { StaffQuotationDetail } from '@/pages/staff/QuotationDetail';
import { StaffSuppliers } from '@/pages/staff/Suppliers';
import { SupplierHome } from '@/pages/supplier/Home';
import { SupplierQuotationPage } from '@/pages/supplier/QuotationBid';

const STAFF: Array<'SERVIDOR' | 'ADMIN'> = ['SERVIDOR', 'ADMIN'];

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/registro" element={<Register />} />
      <Route
        path="/"
        element={
          <RequireRole roles={STAFF}>
            <StaffDashboard />
          </RequireRole>
        }
      />
      <Route
        path="/cotacoes/nova"
        element={
          <RequireRole roles={STAFF}>
            <NewQuotation />
          </RequireRole>
        }
      />
      <Route
        path="/cotacoes/:id"
        element={
          <RequireRole roles={STAFF}>
            <StaffQuotationDetail />
          </RequireRole>
        }
      />
      <Route
        path="/fornecedores"
        element={
          <RequireRole roles={STAFF}>
            <StaffSuppliers />
          </RequireRole>
        }
      />
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
