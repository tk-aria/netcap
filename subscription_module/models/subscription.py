from odoo import models, fields, api
from datetime import date, timedelta
from dateutil.relativedelta import relativedelta


class Subscription(models.Model):
    _name = 'sale.subscription'
    _description = 'Subscription'
    _order = 'name'
    _inherit = ['mail.thread', 'mail.activity.mixin']

    name = fields.Char('サブスクリプション名', required=True, tracking=True)
    partner_id = fields.Many2one('res.partner', string='顧客', required=True, tracking=True)
    company_id = fields.Many2one('res.company', string='会社',
                                  default=lambda self: self.env.company)
    state = fields.Selection([
        ('draft', '下書き'),
        ('active', '有効'),
        ('paused', '一時停止'),
        ('closed', '終了'),
    ], string='ステータス', default='draft', required=True, tracking=True)

    # Billing
    recurring_interval = fields.Integer('請求間隔', default=1, required=True)
    recurring_rule_type = fields.Selection([
        ('monthly', '月次'),
        ('quarterly', '四半期'),
        ('yearly', '年次'),
    ], string='請求サイクル', default='monthly', required=True)
    date_start = fields.Date('開始日', default=fields.Date.today, required=True)
    date_end = fields.Date('終了日')
    next_invoice_date = fields.Date('次回請求日', compute='_compute_next_invoice_date', store=True)

    # Lines
    line_ids = fields.One2many('sale.subscription.line', 'subscription_id', string='明細')

    # Computed
    recurring_total = fields.Float('定期売上', compute='_compute_recurring_total', store=True)
    invoice_count = fields.Integer('請求書数', compute='_compute_invoice_count')
    invoice_ids = fields.One2many('account.move', 'subscription_id', string='請求書')

    # MRR
    mrr = fields.Float('MRR', compute='_compute_mrr', store=True,
                        help='Monthly Recurring Revenue')

    @api.depends('line_ids.price_subtotal')
    def _compute_recurring_total(self):
        for sub in self:
            sub.recurring_total = sum(sub.line_ids.mapped('price_subtotal'))

    @api.depends('recurring_total', 'recurring_rule_type', 'recurring_interval')
    def _compute_mrr(self):
        for sub in self:
            total = sub.recurring_total
            interval = sub.recurring_interval or 1
            if sub.recurring_rule_type == 'monthly':
                sub.mrr = total / interval
            elif sub.recurring_rule_type == 'quarterly':
                sub.mrr = total / (3 * interval)
            elif sub.recurring_rule_type == 'yearly':
                sub.mrr = total / (12 * interval)
            else:
                sub.mrr = total

    @api.depends('date_start', 'recurring_rule_type', 'recurring_interval')
    def _compute_next_invoice_date(self):
        for sub in self:
            if not sub.date_start:
                sub.next_invoice_date = False
                continue
            # Find the next invoice date from today
            current = sub.date_start
            today = fields.Date.today()
            delta = sub._get_recurring_delta()
            while current <= today:
                current += delta
            sub.next_invoice_date = current

    def _get_recurring_delta(self):
        interval = self.recurring_interval or 1
        if self.recurring_rule_type == 'monthly':
            return relativedelta(months=interval)
        elif self.recurring_rule_type == 'quarterly':
            return relativedelta(months=3 * interval)
        elif self.recurring_rule_type == 'yearly':
            return relativedelta(years=interval)
        return relativedelta(months=1)

    def _compute_invoice_count(self):
        for sub in self:
            sub.invoice_count = len(sub.invoice_ids)

    def action_activate(self):
        self.write({'state': 'active'})

    def action_pause(self):
        self.write({'state': 'paused'})

    def action_close(self):
        self.write({'state': 'closed'})

    def action_draft(self):
        self.write({'state': 'draft'})

    def action_create_invoice(self):
        """Manually create invoice for this subscription."""
        invoices = self.env['account.move']
        for sub in self:
            if sub.state != 'active':
                continue
            invoice = sub._create_invoice()
            if invoice:
                invoices |= invoice
        if invoices:
            return {
                'type': 'ir.actions.act_window',
                'name': '請求書',
                'res_model': 'account.move',
                'view_mode': 'list,form',
                'domain': [('id', 'in', invoices.ids)],
            }
        return True

    def _create_invoice(self):
        """Create a single invoice for this subscription."""
        if not self.line_ids:
            return False

        invoice_vals = {
            'move_type': 'out_invoice',
            'partner_id': self.partner_id.id,
            'subscription_id': self.id,
            'invoice_date': fields.Date.today(),
            'invoice_line_ids': [],
        }
        for line in self.line_ids:
            invoice_vals['invoice_line_ids'].append((0, 0, {
                'product_id': line.product_id.id,
                'name': line.name,
                'quantity': line.quantity,
                'price_unit': line.price_unit,
            }))

        invoice = self.env['account.move'].create(invoice_vals)
        return invoice

    def action_view_invoices(self):
        return {
            'type': 'ir.actions.act_window',
            'name': '請求書',
            'res_model': 'account.move',
            'view_mode': 'list,form',
            'domain': [('subscription_id', '=', self.id)],
        }

    @api.model
    def _cron_create_invoices(self):
        """Cron job: auto-create invoices for subscriptions due today."""
        today = fields.Date.today()
        subs = self.search([
            ('state', '=', 'active'),
            ('next_invoice_date', '<=', today),
        ])
        for sub in subs:
            # Check end date
            if sub.date_end and today > sub.date_end:
                sub.action_close()
                continue
            sub._create_invoice()


class SubscriptionLine(models.Model):
    _name = 'sale.subscription.line'
    _description = 'Subscription Line'

    subscription_id = fields.Many2one('sale.subscription', string='サブスクリプション',
                                       required=True, ondelete='cascade')
    product_id = fields.Many2one('product.product', string='商品', required=True)
    name = fields.Char('説明')
    quantity = fields.Float('数量', default=1.0)
    price_unit = fields.Float('単価')
    price_subtotal = fields.Float('小計', compute='_compute_subtotal', store=True)

    @api.depends('quantity', 'price_unit')
    def _compute_subtotal(self):
        for line in self:
            line.price_subtotal = line.quantity * line.price_unit

    @api.onchange('product_id')
    def _onchange_product_id(self):
        if self.product_id:
            self.name = self.product_id.name
            self.price_unit = self.product_id.lst_price


class AccountMove(models.Model):
    _inherit = 'account.move'

    subscription_id = fields.Many2one('sale.subscription', string='サブスクリプション')
