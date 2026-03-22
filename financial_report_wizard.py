from odoo import models, fields, api
from datetime import date


class FinancialReportWizard(models.TransientModel):
    _name = 'financial.report.wizard'
    _description = 'Financial Report Wizard'

    report_type = fields.Selection([
        ('bs', '\u8cb8\u501f\u5bfe\u7167\u8868 (BS)'),
        ('pl', '\u640d\u76ca\u8a08\u7b97\u66f8 (PL)'),
        ('tb', '\u6b8b\u9ad8\u8a66\u7b97\u8868 (TB)'),
    ], string='\u30ec\u30dd\u30fc\u30c8\u7a2e\u985e', default='bs', required=True)
    date_from = fields.Date('\u958b\u59cb\u65e5', default=lambda self: date(date.today().year, 1, 1))
    date_to = fields.Date('\u7d42\u4e86\u65e5', default=fields.Date.today)
    report_html = fields.Html('\u30ec\u30dd\u30fc\u30c8', readonly=True, sanitize=False)

    def action_generate(self):
        self.ensure_one()
        if self.report_type == 'bs':
            html = self._generate_bs()
        elif self.report_type == 'pl':
            html = self._generate_pl()
        else:
            html = self._generate_tb()
        self.report_html = html
        return {
            'type': 'ir.actions.act_window',
            'res_model': 'financial.report.wizard',
            'res_id': self.id,
            'view_mode': 'form',
            'target': 'new',
        }

    def _get_balances(self, date_from=None, date_to=None):
        domain = [('parent_state', '=', 'posted')]
        if date_from:
            domain.append(('date', '>=', date_from))
        if date_to:
            domain.append(('date', '<=', date_to))
        lines = self.env['account.move.line'].search(domain)

        balances = {}
        for line in lines:
            acc = line.account_id
            key = acc.id
            if key not in balances:
                balances[key] = {
                    'code': acc.code,
                    'name': acc.name,
                    'type': acc.account_type,
                    'debit': 0.0,
                    'credit': 0.0,
                }
            balances[key]['debit'] += line.debit
            balances[key]['credit'] += line.credit

        for v in balances.values():
            v['balance'] = v['debit'] - v['credit']
        return balances

    def _fmt(self, amount):
        if amount >= 0:
            return '\u00a5{:,.0f}'.format(amount)
        return '<span style="color:red">\u00a5{:,.0f}</span>'.format(amount)

    def _section_header(self, name):
        return '<tr style="background:#444;color:#fff;"><td colspan="2" style="padding:6px;font-weight:bold;">{}</td></tr>'.format(name)

    def _row(self, label, amount, indent=20):
        return '<tr><td style="padding:4px {}px;">{}</td><td style="text-align:right;padding:4px;">{}</td></tr>'.format(indent, label, self._fmt(amount))

    def _subtotal_row(self, label, amount):
        return '<tr style="border-top:1px solid #666;"><td style="padding:4px 10px;font-weight:bold;">{}</td><td style="text-align:right;padding:4px;font-weight:bold;">{}</td></tr>'.format(label, self._fmt(amount))

    def _total_row(self, label, amount):
        return '<tr style="background:#333;color:#0f0;"><td style="padding:8px;font-weight:bold;">{}</td><td style="text-align:right;padding:8px;font-weight:bold;">{}</td></tr>'.format(label, self._fmt(amount))

    def _generate_bs(self):
        balances = self._get_balances(date_to=self.date_to)
        items = sorted(balances.values(), key=lambda x: x['code'])

        bs_categories = {
            '\u6d41\u52d5\u8cc7\u7523': ['asset_receivable', 'asset_cash', 'asset_current', 'asset_prepayments'],
            '\u56fa\u5b9a\u8cc7\u7523': ['asset_fixed', 'asset_non_current'],
            '\u6d41\u52d5\u8ca0\u50b5': ['liability_payable', 'liability_credit_card', 'liability_current'],
            '\u56fa\u5b9a\u8ca0\u50b5': ['liability_non_current'],
            '\u7d14\u8cc7\u7523': ['equity', 'equity_unaffected'],
        }

        pl_types = ['income', 'income_other', 'expense', 'expense_direct_cost', 'expense_depreciation']
        net_income = -sum(v['balance'] for v in items if v['type'] in pl_types)

        html = '<div style="font-family:sans-serif;padding:20px;max-width:800px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u8cb8\u501f\u5bfe\u7167\u8868 (Balance Sheet)</h2>'
        html += '<p style="text-align:center;">{} \u73fe\u5728</p>'.format(self.date_to)
        html += '<table style="width:100%;border-collapse:collapse;font-size:14px;">'
        html += '<tr style="background:#2d2d2d;color:white;"><th style="text-align:left;padding:8px;">\u52d8\u5b9a\u79d1\u76ee</th><th style="text-align:right;padding:8px;width:150px;">\u91d1\u984d</th></tr>'

        # Assets
        asset_total = 0
        for cat_name in ['\u6d41\u52d5\u8cc7\u7523', '\u56fa\u5b9a\u8cc7\u7523']:
            types = bs_categories[cat_name]
            cat_items = [v for v in items if v['type'] in types and (v['debit'] != 0 or v['credit'] != 0)]
            if cat_items:
                html += self._section_header(cat_name)
                cat_total = 0
                for item in cat_items:
                    html += self._row('{} {}'.format(item['code'], item['name']), item['balance'])
                    cat_total += item['balance']
                html += self._subtotal_row('{}\u5408\u8a08'.format(cat_name), cat_total)
                asset_total += cat_total

        html += self._total_row('\u8cc7\u7523\u5408\u8a08', asset_total)

        # Liabilities
        liab_total = 0
        for cat_name in ['\u6d41\u52d5\u8ca0\u50b5', '\u56fa\u5b9a\u8ca0\u50b5']:
            types = bs_categories[cat_name]
            cat_items = [v for v in items if v['type'] in types and (v['debit'] != 0 or v['credit'] != 0)]
            if cat_items:
                html += self._section_header(cat_name)
                cat_total = 0
                for item in cat_items:
                    bal = -item['balance']
                    html += self._row('{} {}'.format(item['code'], item['name']), bal)
                    cat_total += bal
                html += self._subtotal_row('{}\u5408\u8a08'.format(cat_name), cat_total)
                liab_total += cat_total

        # Equity
        equity_items = [v for v in items if v['type'] in bs_categories['\u7d14\u8cc7\u7523'] and (v['debit'] != 0 or v['credit'] != 0)]
        html += self._section_header('\u7d14\u8cc7\u7523')
        equity_total = 0
        for item in equity_items:
            bal = -item['balance']
            html += self._row('{} {}'.format(item['code'], item['name']), bal)
            equity_total += bal
        html += self._row('\u5f53\u671f\u7d14\u5229\u76ca', net_income)
        equity_total += net_income
        html += self._subtotal_row('\u7d14\u8cc7\u7523\u5408\u8a08', equity_total)

        html += self._total_row('\u8ca0\u50b5\u30fb\u7d14\u8cc7\u7523\u5408\u8a08', liab_total + equity_total)
        html += '</table></div>'
        return html

    def _generate_pl(self):
        balances = self._get_balances(date_from=self.date_from, date_to=self.date_to)
        items = sorted(balances.values(), key=lambda x: x['code'])

        html = '<div style="font-family:sans-serif;padding:20px;max-width:800px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u640d\u76ca\u8a08\u7b97\u66f8 (Profit & Loss)</h2>'
        html += '<p style="text-align:center;">{} \u301c {}</p>'.format(self.date_from, self.date_to)
        html += '<table style="width:100%;border-collapse:collapse;font-size:14px;">'
        html += '<tr style="background:#2d2d2d;color:white;"><th style="text-align:left;padding:8px;">\u52d8\u5b9a\u79d1\u76ee</th><th style="text-align:right;padding:8px;width:150px;">\u91d1\u984d</th></tr>'

        # Revenue
        revenue_total = 0
        rev_items = [v for v in items if v['type'] in ('income', 'income_other') and (v['debit'] != 0 or v['credit'] != 0)]
        if rev_items:
            html += self._section_header('\u58f2\u4e0a\u9ad8')
            for item in rev_items:
                rev = -item['balance']
                html += self._row('{} {}'.format(item['code'], item['name']), rev)
                revenue_total += rev
        html += self._total_row('\u58f2\u4e0a\u9ad8\u5408\u8a08', revenue_total)

        # COGS
        cogs_total = 0
        cogs_items = [v for v in items if v['type'] == 'expense_direct_cost' and (v['debit'] != 0 or v['credit'] != 0)]
        if cogs_items:
            html += self._section_header('\u58f2\u4e0a\u539f\u4fa1')
            for item in cogs_items:
                html += self._row('{} {}'.format(item['code'], item['name']), item['balance'])
                cogs_total += item['balance']

        gross = revenue_total - cogs_total
        html += self._total_row('\u58f2\u4e0a\u7dcf\u5229\u76ca', gross)

        # SGA
        sga_total = 0
        sga_items = [v for v in items if v['type'] in ('expense', 'expense_depreciation') and (v['debit'] != 0 or v['credit'] != 0)]
        if sga_items:
            html += self._section_header('\u8ca9\u58f2\u8cbb\u53ca\u3073\u4e00\u822c\u7ba1\u7406\u8cbb')
            for item in sga_items:
                html += self._row('{} {}'.format(item['code'], item['name']), item['balance'])
                sga_total += item['balance']

        operating = gross - sga_total
        html += self._total_row('\u55b6\u696d\u5229\u76ca', operating)
        html += '<tr style="background:#222;color:#ff0;"><td style="padding:8px;font-weight:bold;font-size:16px;">\u5f53\u671f\u7d14\u5229\u76ca</td><td style="text-align:right;padding:8px;font-weight:bold;font-size:16px;">{}</td></tr>'.format(self._fmt(operating))
        html += '</table></div>'
        return html

    def _generate_tb(self):
        balances = self._get_balances(date_to=self.date_to)
        items = sorted(balances.values(), key=lambda x: x['code'])

        html = '<div style="font-family:sans-serif;padding:20px;max-width:900px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u6b8b\u9ad8\u8a66\u7b97\u8868 (Trial Balance)</h2>'
        html += '<p style="text-align:center;">{} \u73fe\u5728</p>'.format(self.date_to)
        html += '<table style="width:100%;border-collapse:collapse;font-size:14px;">'
        html += '<tr style="background:#2d2d2d;color:white;">'
        html += '<th style="text-align:left;padding:8px;">\u30b3\u30fc\u30c9</th>'
        html += '<th style="text-align:left;padding:8px;">\u52d8\u5b9a\u79d1\u76ee</th>'
        html += '<th style="text-align:right;padding:8px;">\u501f\u65b9</th>'
        html += '<th style="text-align:right;padding:8px;">\u8cb8\u65b9</th>'
        html += '<th style="text-align:right;padding:8px;">\u6b8b\u9ad8</th></tr>'

        total_debit = 0
        total_credit = 0
        for item in items:
            if item['debit'] == 0 and item['credit'] == 0:
                continue
            html += '<tr>'
            html += '<td style="padding:4px 8px;">{}</td>'.format(item['code'])
            html += '<td style="padding:4px 8px;">{}</td>'.format(item['name'])
            html += '<td style="text-align:right;padding:4px 8px;">{}</td>'.format(self._fmt(item['debit']))
            html += '<td style="text-align:right;padding:4px 8px;">{}</td>'.format(self._fmt(item['credit']))
            html += '<td style="text-align:right;padding:4px 8px;">{}</td>'.format(self._fmt(item['balance']))
            html += '</tr>'
            total_debit += item['debit']
            total_credit += item['credit']

        html += '<tr style="background:#333;color:#0f0;border-top:2px solid #666;">'
        html += '<td colspan="2" style="padding:8px;font-weight:bold;">\u5408\u8a08</td>'
        html += '<td style="text-align:right;padding:8px;font-weight:bold;">{}</td>'.format(self._fmt(total_debit))
        html += '<td style="text-align:right;padding:8px;font-weight:bold;">{}</td>'.format(self._fmt(total_credit))
        html += '<td style="text-align:right;padding:8px;font-weight:bold;">{}</td>'.format(self._fmt(total_debit - total_credit))
        html += '</tr></table></div>'
        return html
