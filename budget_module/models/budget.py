from odoo import models, fields, api
from datetime import date


class Budget(models.Model):
    _name = 'budget.budget'
    _description = 'Budget'
    _order = 'date_from desc'

    name = fields.Char('Budget Name', required=True)
    date_from = fields.Date('Start Date', required=True, default=lambda self: date(date.today().year, 1, 1))
    date_to = fields.Date('End Date', required=True, default=lambda self: date(date.today().year, 12, 31))
    state = fields.Selection([
        ('draft', 'Draft'),
        ('confirmed', 'Confirmed'),
        ('done', 'Done'),
        ('cancelled', 'Cancelled'),
    ], string='Status', default='draft', required=True)
    line_ids = fields.One2many('budget.line', 'budget_id', string='Budget Lines')
    company_id = fields.Many2one('res.company', string='Company',
                                 default=lambda self: self.env.company)
    total_planned = fields.Float('Total Planned', compute='_compute_totals', store=True)
    total_actual = fields.Float('Total Actual', compute='_compute_totals', store=True)
    total_variance = fields.Float('Total Variance', compute='_compute_totals', store=True)
    total_percentage = fields.Float('Achievement %', compute='_compute_totals', store=True)

    @api.depends('line_ids.planned_amount', 'line_ids.actual_amount')
    def _compute_totals(self):
        for budget in self:
            budget.total_planned = sum(budget.line_ids.mapped('planned_amount'))
            budget.total_actual = sum(budget.line_ids.mapped('actual_amount'))
            budget.total_variance = budget.total_actual - budget.total_planned
            budget.total_percentage = (
                (budget.total_actual / budget.total_planned * 100)
                if budget.total_planned else 0.0
            )

    def action_confirm(self):
        self.write({'state': 'confirmed'})

    def action_done(self):
        self.write({'state': 'done'})

    def action_draft(self):
        self.write({'state': 'draft'})

    def action_cancel(self):
        self.write({'state': 'cancelled'})

    def action_compute_actual(self):
        for budget in self:
            for line in budget.line_ids:
                line._compute_actual_amount()


class BudgetLine(models.Model):
    _name = 'budget.line'
    _description = 'Budget Line'
    _order = 'account_id'

    budget_id = fields.Many2one('budget.budget', string='Budget', required=True, ondelete='cascade')
    account_id = fields.Many2one('account.account', string='Account', required=True)
    analytic_account_id = fields.Many2one('account.analytic.account', string='Analytic Account')
    date_from = fields.Date('Start Date', related='budget_id.date_from', store=True)
    date_to = fields.Date('End Date', related='budget_id.date_to', store=True)
    planned_amount = fields.Float('Planned Amount', required=True)
    actual_amount = fields.Float('Actual Amount', compute='_compute_actual_amount', store=True)
    variance = fields.Float('Variance', compute='_compute_variance', store=True)
    percentage = fields.Float('Achievement %', compute='_compute_variance', store=True)
    company_id = fields.Many2one('res.company', related='budget_id.company_id', store=True)

    @api.depends('budget_id.date_from', 'budget_id.date_to', 'account_id', 'analytic_account_id')
    def _compute_actual_amount(self):
        for line in self:
            if not line.account_id or not line.date_from or not line.date_to:
                line.actual_amount = 0.0
                continue
            domain = [
                ('account_id', '=', line.account_id.id),
                ('date', '>=', line.date_from),
                ('date', '<=', line.date_to),
                ('parent_state', '=', 'posted'),
            ]
            if line.analytic_account_id:
                domain.append(('analytic_distribution', 'like', str(line.analytic_account_id.id)))
            move_lines = self.env['account.move.line'].search(domain)
            # For income accounts, actual = credit - debit (positive = good)
            # For expense accounts, actual = debit - credit (positive = spent)
            account_type = line.account_id.account_type
            if account_type in ('income', 'income_other'):
                line.actual_amount = sum(move_lines.mapped('credit')) - sum(move_lines.mapped('debit'))
            else:
                line.actual_amount = sum(move_lines.mapped('debit')) - sum(move_lines.mapped('credit'))

    @api.depends('planned_amount', 'actual_amount')
    def _compute_variance(self):
        for line in self:
            line.variance = line.actual_amount - line.planned_amount
            line.percentage = (
                (line.actual_amount / line.planned_amount * 100)
                if line.planned_amount else 0.0
            )
