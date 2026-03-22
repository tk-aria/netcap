from odoo import models, fields, api
from datetime import date, timedelta


class FinancialReportWizard(models.TransientModel):
    _name = 'financial.report.wizard'
    _description = 'Financial Report Wizard'

    report_type = fields.Selection([
        ('bs', '\u8cb8\u501f\u5bfe\u7167\u8868 (BS)'),
        ('pl', '\u640d\u76ca\u8a08\u7b97\u66f8 (PL)'),
        ('tb', '\u6b8b\u9ad8\u8a66\u7b97\u8868 (TB)'),
        ('gl', '\u7dcf\u52d8\u5b9a\u5143\u5e33 (GL)'),
        ('ar', '\u58f2\u639b\u91d1\u5e74\u9f62\u8868'),
        ('ap', '\u8cb7\u639b\u91d1\u5e74\u9f62\u8868'),
    ], string='\u30ec\u30dd\u30fc\u30c8\u7a2e\u985e', default='bs', required=True)
    date_from = fields.Date('\u958b\u59cb\u65e5', default=lambda self: date(date.today().year, 1, 1))
    date_to = fields.Date('\u7d42\u4e86\u65e5', default=fields.Date.today)
    account_id = fields.Many2one('account.account', string='\u52d8\u5b9a\u79d1\u76ee',
                                 help='GL: filter by specific account')
    report_html = fields.Html('\u30ec\u30dd\u30fc\u30c8', readonly=True, sanitize=False)

    def action_generate(self):
        self.ensure_one()
        generators = {
            'bs': self._generate_bs,
            'pl': self._generate_pl,
            'tb': self._generate_tb,
            'gl': self._generate_gl,
            'ar': self._generate_aged_receivable,
            'ap': self._generate_aged_payable,
        }
        self.report_html = generators[self.report_type]()
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

    def _section_header(self, name, colspan=2):
        return '<tr style="background:#444;color:#fff;"><td colspan="{}" style="padding:6px;font-weight:bold;">{}</td></tr>'.format(colspan, name)

    def _row(self, label, amount, indent=20):
        return '<tr><td style="padding:4px {}px;">{}</td><td style="text-align:right;padding:4px;">{}</td></tr>'.format(indent, label, self._fmt(amount))

    def _subtotal_row(self, label, amount):
        return '<tr style="border-top:1px solid #666;"><td style="padding:4px 10px;font-weight:bold;">{}</td><td style="text-align:right;padding:4px;font-weight:bold;">{}</td></tr>'.format(label, self._fmt(amount))

    def _total_row(self, label, amount):
        return '<tr style="background:#333;color:#0f0;"><td style="padding:8px;font-weight:bold;">{}</td><td style="text-align:right;padding:8px;font-weight:bold;">{}</td></tr>'.format(label, self._fmt(amount))

    def _table_start(self, headers):
        html = '<table style="width:100%;border-collapse:collapse;font-size:14px;">'
        html += '<tr style="background:#2d2d2d;color:white;">'
        for h in headers:
            align = 'right' if h.get('align') == 'right' else 'left'
            w = ' width:{}'.format(h['width']) if h.get('width') else ''
            html += '<th style="text-align:{};padding:8px;{}">{}</th>'.format(align, w, h['label'])
        html += '</tr>'
        return html

    # ====== BS ======
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

        def build_side_rows(categories, negate=False):
            rows = []
            side_total = 0
            for cat_name, types in categories:
                cat_items = [v for v in items if v['type'] in types and (v['debit'] != 0 or v['credit'] != 0)]
                if cat_items:
                    rows.append(('header', cat_name))
                    cat_total = 0
                    for item in cat_items:
                        bal = -item['balance'] if negate else item['balance']
                        rows.append(('item', '{} {}'.format(item['code'], item['name']), bal))
                        cat_total += bal
                    rows.append(('subtotal', '{}\u5408\u8a08'.format(cat_name), cat_total))
                    side_total += cat_total
            return rows, side_total

        # Build left side (assets)
        asset_rows, asset_total = build_side_rows([
            ('\u6d41\u52d5\u8cc7\u7523', bs_categories['\u6d41\u52d5\u8cc7\u7523']),
            ('\u56fa\u5b9a\u8cc7\u7523', bs_categories['\u56fa\u5b9a\u8cc7\u7523']),
        ])

        # Build right side (liabilities + equity)
        liab_rows, liab_total = build_side_rows([
            ('\u6d41\u52d5\u8ca0\u50b5', bs_categories['\u6d41\u52d5\u8ca0\u50b5']),
            ('\u56fa\u5b9a\u8ca0\u50b5', bs_categories['\u56fa\u5b9a\u8ca0\u50b5']),
        ], negate=True)

        # Add equity section
        equity_items = [v for v in items if v['type'] in bs_categories['\u7d14\u8cc7\u7523'] and (v['debit'] != 0 or v['credit'] != 0)]
        equity_rows = [('header', '\u7d14\u8cc7\u7523')]
        equity_total = 0
        for item in equity_items:
            bal = -item['balance']
            equity_rows.append(('item', '{} {}'.format(item['code'], item['name']), bal))
            equity_total += bal
        equity_rows.append(('item', '\u5f53\u671f\u7d14\u5229\u76ca', net_income))
        equity_total += net_income
        equity_rows.append(('subtotal', '\u7d14\u8cc7\u7523\u5408\u8a08', equity_total))

        right_rows = liab_rows + equity_rows
        right_total = liab_total + equity_total

        # Pad shorter side
        max_rows = max(len(asset_rows), len(right_rows))
        while len(asset_rows) < max_rows:
            asset_rows.append(('empty',))
        while len(right_rows) < max_rows:
            right_rows.append(('empty',))

        # Render HTML
        hdr = 'background:#444;color:#fff;padding:6px;font-weight:bold;'
        sub = 'border-top:1px solid #666;padding:4px 10px;font-weight:bold;'
        tot = 'background:#333;color:#0f0;padding:8px;font-weight:bold;'
        cell_l = 'padding:4px 8px;border-right:2px solid #555;'
        cell_r = 'padding:4px 8px;'
        amt_l = 'text-align:right;padding:4px 8px;border-right:2px solid #555;'
        amt_r = 'text-align:right;padding:4px 8px;'

        html = '<div style="font-family:sans-serif;padding:20px;max-width:1100px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u8cb8\u501f\u5bfe\u7167\u8868 (Balance Sheet)</h2>'
        html += '<p style="text-align:center;">{} \u73fe\u5728</p>'.format(self.date_to)
        html += '<table style="width:100%;border-collapse:collapse;font-size:14px;">'
        html += '<tr style="background:#2d2d2d;color:white;">'
        html += '<th colspan="2" style="text-align:center;padding:8px;border-right:2px solid #555;width:50%;">\u8cc7\u7523\u306e\u90e8</th>'
        html += '<th colspan="2" style="text-align:center;padding:8px;width:50%;">\u8ca0\u50b5\u53ca\u3073\u8cc7\u672c\u306e\u90e8</th>'
        html += '</tr>'

        def render_cell(row, side):
            s_cell = cell_l if side == 'left' else cell_r
            s_amt = amt_l if side == 'left' else amt_r
            if row[0] == 'header':
                return '<td colspan="2" style="{}{}">'.format(hdr, 'border-right:2px solid #555;' if side == 'left' else '') + row[1] + '</td>'
            elif row[0] == 'item':
                return '<td style="{}">{}</td><td style="{}">{}</td>'.format(
                    s_cell if side == 'left' else cell_r, row[1],
                    s_amt if side == 'left' else amt_r, self._fmt(row[2]))
            elif row[0] == 'subtotal':
                return '<td style="{}{}">{}</td><td style="{}{}">{}</td>'.format(
                    sub, 'border-right:2px solid #555;' if side == 'left' else '', row[1],
                    sub, 'border-right:2px solid #555;' if side == 'left' else '', self._fmt(row[2]))
            else:
                br = 'border-right:2px solid #555;' if side == 'left' else ''
                return '<td style="padding:4px;{}"></td><td style="padding:4px;{}"></td>'.format(br, br)

        for i in range(max_rows):
            html += '<tr>'
            html += render_cell(asset_rows[i], 'left')
            html += render_cell(right_rows[i], 'right')
            html += '</tr>'

        # Total row
        html += '<tr>'
        html += '<td style="{}border-right:2px solid #555;">\u8cc7\u7523\u5408\u8a08</td>'.format(tot)
        html += '<td style="{}text-align:right;border-right:2px solid #555;">{}</td>'.format(tot, self._fmt(asset_total))
        html += '<td style="{}">\u8ca0\u50b5\u30fb\u8cc7\u672c\u5408\u8a08</td>'.format(tot)
        html += '<td style="{}text-align:right;">{}</td>'.format(tot, self._fmt(right_total))
        html += '</tr>'

        html += '</table></div>'
        return html

    # ====== PL ======
    def _generate_pl(self):
        balances = self._get_balances(date_from=self.date_from, date_to=self.date_to)
        items = sorted(balances.values(), key=lambda x: x['code'])

        html = '<div style="font-family:sans-serif;padding:20px;max-width:800px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u640d\u76ca\u8a08\u7b97\u66f8 (Profit & Loss)</h2>'
        html += '<p style="text-align:center;">{} \u301c {}</p>'.format(self.date_from, self.date_to)
        html += self._table_start([{'label': '\u52d8\u5b9a\u79d1\u76ee'}, {'label': '\u91d1\u984d', 'align': 'right', 'width': '150px'}])

        revenue_total = 0
        rev_items = [v for v in items if v['type'] in ('income', 'income_other') and (v['debit'] != 0 or v['credit'] != 0)]
        if rev_items:
            html += self._section_header('\u58f2\u4e0a\u9ad8')
            for item in rev_items:
                rev = -item['balance']
                html += self._row('{} {}'.format(item['code'], item['name']), rev)
                revenue_total += rev
        html += self._total_row('\u58f2\u4e0a\u9ad8\u5408\u8a08', revenue_total)

        cogs_total = 0
        cogs_items = [v for v in items if v['type'] == 'expense_direct_cost' and (v['debit'] != 0 or v['credit'] != 0)]
        if cogs_items:
            html += self._section_header('\u58f2\u4e0a\u539f\u4fa1')
            for item in cogs_items:
                html += self._row('{} {}'.format(item['code'], item['name']), item['balance'])
                cogs_total += item['balance']

        gross = revenue_total - cogs_total
        html += self._total_row('\u58f2\u4e0a\u7dcf\u5229\u76ca', gross)

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

    # ====== TB ======
    def _generate_tb(self):
        balances = self._get_balances(date_to=self.date_to)
        items = sorted(balances.values(), key=lambda x: x['code'])

        html = '<div style="font-family:sans-serif;padding:20px;max-width:900px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u6b8b\u9ad8\u8a66\u7b97\u8868 (Trial Balance)</h2>'
        html += '<p style="text-align:center;">{} \u73fe\u5728</p>'.format(self.date_to)
        html += self._table_start([
            {'label': '\u30b3\u30fc\u30c9'}, {'label': '\u52d8\u5b9a\u79d1\u76ee'},
            {'label': '\u501f\u65b9', 'align': 'right'}, {'label': '\u8cb8\u65b9', 'align': 'right'},
            {'label': '\u6b8b\u9ad8', 'align': 'right'}
        ])

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

    # ====== GL (General Ledger) ======
    def _generate_gl(self):
        domain = [('parent_state', '=', 'posted')]
        if self.date_from:
            domain.append(('date', '>=', self.date_from))
        if self.date_to:
            domain.append(('date', '<=', self.date_to))
        if self.account_id:
            domain.append(('account_id', '=', self.account_id.id))

        move_lines = self.env['account.move.line'].search(domain, order='account_id, date, id')

        html = '<div style="font-family:sans-serif;padding:20px;max-width:1000px;margin:auto;">'
        html += '<h2 style="text-align:center;">\u7dcf\u52d8\u5b9a\u5143\u5e33 (General Ledger)</h2>'
        html += '<p style="text-align:center;">{} \u301c {}</p>'.format(self.date_from, self.date_to)

        current_account = None
        running_balance = 0.0

        for line in move_lines:
            if line.account_id != current_account:
                if current_account is not None:
                    html += '<tr style="background:#333;color:#0f0;"><td colspan="3" style="padding:4px 8px;font-weight:bold;">\u6b8b\u9ad8</td>'
                    html += '<td style="text-align:right;padding:4px 8px;font-weight:bold;">{}</td>'.format(self._fmt(running_balance))
                    html += '</tr></table><br/>'

                current_account = line.account_id
                running_balance = 0.0
                html += self._table_start([
                    {'label': '{} {}'.format(current_account.code, current_account.name)},
                    {'label': '\u501f\u65b9', 'align': 'right', 'width': '120px'},
                    {'label': '\u8cb8\u65b9', 'align': 'right', 'width': '120px'},
                    {'label': '\u6b8b\u9ad8', 'align': 'right', 'width': '120px'},
                ])

            running_balance += line.debit - line.credit
            ref = line.move_id.name or ''
            label = line.name or ''
            display = '{} {} {}'.format(line.date, ref, label)
            html += '<tr>'
            html += '<td style="padding:3px 8px;font-size:12px;">{}</td>'.format(display)
            html += '<td style="text-align:right;padding:3px 8px;">{}</td>'.format(self._fmt(line.debit) if line.debit else '')
            html += '<td style="text-align:right;padding:3px 8px;">{}</td>'.format(self._fmt(line.credit) if line.credit else '')
            html += '<td style="text-align:right;padding:3px 8px;">{}</td>'.format(self._fmt(running_balance))
            html += '</tr>'

        if current_account is not None:
            html += '<tr style="background:#333;color:#0f0;"><td colspan="3" style="padding:4px 8px;font-weight:bold;">\u6b8b\u9ad8</td>'
            html += '<td style="text-align:right;padding:4px 8px;font-weight:bold;">{}</td>'.format(self._fmt(running_balance))
            html += '</tr></table>'

        html += '</div>'
        return html

    # ====== Aged Receivable ======
    def _generate_aged_receivable(self):
        return self._generate_aged_report('asset_receivable', '\u58f2\u639b\u91d1\u5e74\u9f62\u8868 (Aged Receivable)')

    # ====== Aged Payable ======
    def _generate_aged_payable(self):
        return self._generate_aged_report('liability_payable', '\u8cb7\u639b\u91d1\u5e74\u9f62\u8868 (Aged Payable)')

    def _generate_aged_report(self, account_type, title):
        today = self.date_to or fields.Date.today()
        periods = [
            ('\u672a\u5230\u6765', None, today),
            ('0-30\u65e5', 1, 30),
            ('31-60\u65e5', 31, 60),
            ('61-90\u65e5', 61, 90),
            ('91-120\u65e5', 91, 120),
            ('120\u65e5\u8d85', 121, None),
        ]

        # Get all unreconciled lines for the account type
        domain = [
            ('account_id.account_type', '=', account_type),
            ('parent_state', '=', 'posted'),
            ('reconciled', '=', False),
            ('date', '<=', today),
        ]
        move_lines = self.env['account.move.line'].search(domain, order='partner_id, date')

        # Group by partner
        partner_data = {}
        for line in move_lines:
            partner_name = line.partner_id.name if line.partner_id else '\u672a\u8a2d\u5b9a'
            if partner_name not in partner_data:
                partner_data[partner_name] = {p[0]: 0.0 for p in periods}
                partner_data[partner_name]['total'] = 0.0

            if account_type == 'asset_receivable':
                amount = line.debit - line.credit
            else:
                amount = line.credit - line.debit

            days = (today - line.date).days

            if days <= 0:
                partner_data[partner_name]['\u672a\u5230\u6765'] += amount
            elif days <= 30:
                partner_data[partner_name]['0-30\u65e5'] += amount
            elif days <= 60:
                partner_data[partner_name]['31-60\u65e5'] += amount
            elif days <= 90:
                partner_data[partner_name]['61-90\u65e5'] += amount
            elif days <= 120:
                partner_data[partner_name]['91-120\u65e5'] += amount
            else:
                partner_data[partner_name]['120\u65e5\u8d85'] += amount
            partner_data[partner_name]['total'] += amount

        html = '<div style="font-family:sans-serif;padding:20px;max-width:1100px;margin:auto;">'
        html += '<h2 style="text-align:center;">{}</h2>'.format(title)
        html += '<p style="text-align:center;">{} \u73fe\u5728</p>'.format(today)

        headers = [{'label': '\u53d6\u5f15\u5148'}]
        for p in periods:
            headers.append({'label': p[0], 'align': 'right', 'width': '100px'})
        headers.append({'label': '\u5408\u8a08', 'align': 'right', 'width': '120px'})
        html += self._table_start(headers)

        grand_totals = {p[0]: 0.0 for p in periods}
        grand_totals['total'] = 0.0

        for partner_name in sorted(partner_data.keys()):
            data = partner_data[partner_name]
            if abs(data['total']) < 0.01:
                continue
            html += '<tr>'
            html += '<td style="padding:4px 8px;">{}</td>'.format(partner_name)
            for p in periods:
                val = data[p[0]]
                html += '<td style="text-align:right;padding:4px 8px;">{}</td>'.format(self._fmt(val) if val else '')
                grand_totals[p[0]] += val
            html += '<td style="text-align:right;padding:4px 8px;font-weight:bold;">{}</td>'.format(self._fmt(data['total']))
            grand_totals['total'] += data['total']
            html += '</tr>'

        html += '<tr style="background:#333;color:#0f0;border-top:2px solid #666;">'
        html += '<td style="padding:8px;font-weight:bold;">\u5408\u8a08</td>'
        for p in periods:
            html += '<td style="text-align:right;padding:8px;font-weight:bold;">{}</td>'.format(self._fmt(grand_totals[p[0]]))
        html += '<td style="text-align:right;padding:8px;font-weight:bold;">{}</td>'.format(self._fmt(grand_totals['total']))
        html += '</tr></table></div>'
        return html
