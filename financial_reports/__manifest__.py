{
    'name': '財務諸表レポート',
    'version': '18.0.1.0.0',
    'category': 'Accounting',
    'summary': 'BS/PL/TB/GL/売掛金・買掛金年齢表',
    'depends': ['account'],
    'data': [
        'security/ir.model.access.csv',
        'views/financial_report_wizard_views.xml',
        'views/menu.xml',
    ],
    'installable': True,
    'application': False,
    'license': 'LGPL-3',
}
